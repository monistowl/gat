use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use gat_cli::cli::GeoCommands;
use gat_io::importers;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::parse_partitions;
use gat_algo::geo_join::perform_spatial_join;

/// Handle `gat geo join` command: map buses/feeders to spatial polygons.
///
/// **Purpose:** Performs spatial joins between power grid topology (buses with lat/lon coordinates)
/// and GIS polygon layers (census tracts, zip codes, planning areas, etc.). Produces polygon_id ↔ bus_id
/// mapping tables for downstream spatial aggregation and planning workflows.
///
/// **Spatial Join Context:**
/// In grid planning and operations, we often need to aggregate grid metrics (load, voltage, reliability)
/// to administrative or planning boundaries (city neighborhoods, utility service territories, regulatory
/// zones). This requires mapping point geometries (buses) to polygon geometries (boundaries).
///
/// **Three Spatial Join Methods:**
///
/// 1. **Point-in-Polygon (default):**
///    - Tests whether bus coordinate (lat, lon) falls inside polygon boundary
///    - Most intuitive and accurate when buses are clearly within a single polygon
///    - Algorithm: Ray casting or winding number (O(n·m) where n=buses, m=polygon vertices)
///    - **Limitation**: Buses exactly on polygon boundaries can be ambiguous
///
/// 2. **Voronoi Tessellation:**
///    - Computes Voronoi diagram from polygon centroids, assigns buses to nearest centroid's polygon
///    - Useful when polygons don't cover entire area or bus coordinates have geospatial errors
///    - Algorithm: Delaunay triangulation → Voronoi dual (O(n log n))
///    - **Limitation**: Ignores actual polygon shapes, only uses centroids
///
/// 3. **K-Nearest Neighbors (KNN):**
///    - Assigns each bus to k nearest polygon centroids by Euclidean distance
///    - Supports multi-polygon assignments (e.g., buses on feeder boundaries serving multiple tracts)
///    - Algorithm: Ball tree or KD-tree spatial index (O(n log m))
///    - **Limitation**: Distance in lat/lon degrees may not reflect actual service area
///
/// **Coordinate Systems and Projections:**
/// - Input coordinates are typically WGS84 (EPSG:4326) lat/lon in decimal degrees
/// - For accurate distance calculations, should project to equal-area CRS (e.g., Albers Equal Area)
/// - Distance in degrees: 1° latitude ≈ 111 km, 1° longitude ≈ 111 km × cos(latitude)
/// - For local US grids, State Plane or UTM projections minimize distortion
///
/// **GeoParquet Format:**
/// GeoParquet extends Parquet with WKB (Well-Known Binary) geometry encoding and spatial metadata.
/// Compatible with GDAL, GeoPandas, DuckDB Spatial, and other GIS tools. See:
/// - GeoParquet spec: https://github.com/opengeospatial/geoparquet
/// - doi:10.3390/ijgi9020102 for spatial joins in energy systems GIS
///
/// **Workflow Example:**
/// ```bash
/// # 1. Export grid topology with lat/lon coordinates
/// gat import matpower --m ./data/case300.m --output ./data/grid.arrow
///
/// # 2. Download census tracts shapefile (e.g., from US Census TIGER/Line)
/// # Convert to GeoParquet: ogr2ogr -f Parquet tracts.parquet tl_2020_06_tract.shp
///
/// # 3. Perform spatial join: map buses to tracts
/// gat geo join \
///   --grid-file ./data/grid.arrow \
///   --polygons ./data/tracts.parquet \
///   --method point_in_polygon \
///   --out ./outputs/bus_to_tract.parquet
///
/// # 4. Use mapping for downstream spatial aggregation
/// # e.g., aggregate load by tract, compute tract-level reliability metrics
/// ```
///
/// **Output Schema:**
/// - bus_id: Grid bus identifier (integer)
/// - bus_lat, bus_lon: Bus coordinates (decimal degrees)
/// - polygon_id: GIS polygon identifier (string or integer from GIS layer)
/// - distance_km: Distance from bus to polygon centroid (useful for knn method, 0.0 for point_in_polygon)
/// - polygon_name: Optional human-readable polygon name (e.g., "Mission District", "ZIP 94103")
///
/// **Real-World Applications:**
/// - **Load Forecasting**: Aggregate historical load by census tract, join with demographic data (population,
///   employment, housing units) for spatial econometric load models. See doi:10.1016/j.energy.2020.117515.
/// - **Reliability Planning**: Compute SAIDI/SAIFI by neighborhood to identify equity gaps and prioritize
///   undergrounding/automation investments. PG&E uses tract-level reliability for CPUC reporting.
/// - **DER Hosting Capacity**: Map hosting capacity results to zip codes for customer-facing hosting capacity maps.
///   California Rule 21 requires utilities to publish hosting capacity by address/zip code.
/// - **Resilience Metrics**: Aggregate critical customer counts (hospitals, fire stations) by service territory
///   for extreme weather preparedness. ConEd uses this for NYC flood resilience planning.
///
/// **Pedagogical Note for Grad Students:**
/// Spatial joins are a fundamental GIS operation, analogous to relational database joins but with geometric
/// predicates (intersects, contains, within) instead of equality. The computational challenge is avoiding
/// O(n²) pairwise distance computations. Modern implementations use spatial indexes (R-tree, Quad-tree) to
/// prune search space. PostGIS, GeoPandas, and DuckDB Spatial all use GEOS library (C++ port of JTS) for
/// robust geometric predicates. For production systems processing millions of buses/polygons, consider
/// distributed spatial joins via Apache Sedona or GeoSpark on Spark.
pub fn handle(command: &GeoCommands) -> Result<()> {
    let GeoCommands::Join {
        grid_file,
        polygons,
        method,
        k,
        out,
        out_partitions,
    } = command
    else {
        unreachable!();
    };

    let partitions = parse_partitions(out_partitions.as_ref());
    let start = Instant::now();

    let res = (|| -> Result<()> {
        // Load grid topology (buses with lat/lon)
        let network = importers::load_grid_from_arrow(grid_file)?;

        // Perform spatial join
        let summary = perform_spatial_join(
            &network,
            Path::new(polygons),
            method,
            *k,
            Path::new(out),
            &partitions,
        )?;

        // Print summary statistics
        println!(
            "Spatial join completed: {} buses × {} polygons using {} method",
            summary.num_buses, summary.num_polygons, method
        );
        println!(
            "  Mapped: {} buses, Unmapped: {} buses",
            summary.num_mapped, summary.num_unmapped
        );
        if summary.num_unmapped > 0 {
            println!(
                "  ⚠️  {} buses outside all polygons (will have null polygon_id)",
                summary.num_unmapped
            );
        }
        println!("  Output: {}", out);

        Ok(())
    })();

    // Record run telemetry
    let params = [
        ("grid_file".to_string(), grid_file.to_string()),
        ("polygons".to_string(), polygons.to_string()),
        ("method".to_string(), method.to_string()),
        ("k".to_string(), k.to_string()),
        ("out".to_string(), out.to_string()),
        (
            "out_partitions".to_string(),
            out_partitions.as_deref().unwrap_or("").to_string(),
        ),
    ];

    let param_refs: Vec<(&str, &str)> = params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    record_run_timed(out, "geo join", &param_refs, start, &res);
    res
}
