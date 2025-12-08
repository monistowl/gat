/**
 * GAT Bridge - JavaScript layer for WASM ↔ Arrow JS ↔ DuckDB-WASM interop
 *
 * This module provides:
 * - Arrow IPC parsing from WASM results
 * - DuckDB-WASM integration for SQL queries on OPF results
 * - DataFrame utilities for visualization
 *
 * Dependencies (loaded via CDN):
 * - Apache Arrow JS: https://cdn.jsdelivr.net/npm/apache-arrow@18/+esm
 * - DuckDB-WASM: https://cdn.jsdelivr.net/npm/@duckdb/duckdb-wasm/+esm
 */

// Arrow JS import - loaded dynamically
let Arrow = null;
let duckdb = null;
let db = null;

/**
 * Initialize the bridge with Arrow JS
 * @returns {Promise<void>}
 */
export async function initArrow() {
    if (!Arrow) {
        // Dynamic import of Apache Arrow from CDN
        Arrow = await import('https://cdn.jsdelivr.net/npm/apache-arrow@18/+esm');
        console.log('[GAT Bridge] Apache Arrow JS loaded, version:', Arrow.version || '18.x');
    }
    return Arrow;
}

/**
 * Initialize DuckDB-WASM for SQL queries
 * Uses locally hosted files to avoid CORS issues with CDN workers
 * @returns {Promise<Object>} DuckDB database instance
 */
export async function initDuckDB() {
    if (db) return db;

    // Dynamic import of DuckDB-WASM
    if (!duckdb) {
        duckdb = await import('https://cdn.jsdelivr.net/npm/@duckdb/duckdb-wasm@1.29.0/+esm');
    }

    // Get base path for local assets (works for both /experimental/ and nested paths)
    const basePath = new URL('.', import.meta.url).href;

    // Configure DuckDB bundles with LOCAL paths for workers (avoids CORS issues)
    // WASM modules can still come from CDN since they're loaded via fetch
    const DUCKDB_BUNDLES = {
        mvp: {
            mainModule: `${basePath}vendor/duckdb/duckdb-mvp.wasm`,
            mainWorker: `${basePath}vendor/duckdb/duckdb-browser-mvp.worker.js`,
        },
        eh: {
            mainModule: `${basePath}vendor/duckdb/duckdb-eh.wasm`,
            mainWorker: `${basePath}vendor/duckdb/duckdb-browser-eh.worker.js`,
        },
    };

    // Select best bundle for this browser
    const bundle = await duckdb.selectBundle(DUCKDB_BUNDLES);
    console.log('[GAT Bridge] Using DuckDB bundle:', bundle.mainWorker);

    const worker = new Worker(bundle.mainWorker);
    const logger = new duckdb.ConsoleLogger();

    db = new duckdb.AsyncDuckDB(logger, worker);
    await db.instantiate(bundle.mainModule);
    console.log('[GAT Bridge] DuckDB-WASM initialized');

    return db;
}

/**
 * Parse Arrow IPC bytes into an Arrow Table
 * @param {Uint8Array} ipcBytes - Arrow IPC stream bytes from WASM
 * @returns {Object} Arrow Table
 */
export function parseArrowIPC(ipcBytes) {
    if (!Arrow) {
        throw new Error('Arrow JS not initialized. Call initArrow() first.');
    }
    return Arrow.tableFromIPC(ipcBytes);
}

/**
 * Run OPF and get results as Arrow Tables
 * @param {Object} wasm - GAT WASM module
 * @param {string} content - MATPOWER case content
 * @param {string} method - 'dc' or 'socp'
 * @returns {Promise<Object>} Object with Arrow Tables for generators, buses, branches, and summary JSON
 */
export async function runOpfWithArrow(wasm, content, method = 'dc') {
    await initArrow();

    // Run appropriate OPF method
    const result = method === 'socp'
        ? wasm.run_socp_opf_arrow(content)
        : wasm.run_dc_opf_arrow(content);

    // Parse Arrow IPC bytes into Tables
    const tables = {
        generators: parseArrowIPC(result.generators),
        buses: parseArrowIPC(result.buses),
        branches: parseArrowIPC(result.branches),
        summary: JSON.parse(result.summary),
    };

    // Free WASM memory
    result.free();

    return tables;
}

/**
 * Convert Arrow Table to plain JavaScript objects
 * @param {Object} table - Arrow Table
 * @returns {Array<Object>} Array of row objects
 */
export function tableToObjects(table) {
    const rows = [];
    for (const row of table) {
        const obj = {};
        for (const field of table.schema.fields) {
            obj[field.name] = row[field.name];
        }
        rows.push(obj);
    }
    return rows;
}

/**
 * Register OPF results in DuckDB for SQL queries
 * @param {Object} tables - Result from runOpfWithArrow
 * @param {string} prefix - Table name prefix (default: 'opf')
 * @returns {Promise<Object>} DuckDB connection
 */
export async function registerInDuckDB(tables, prefix = 'opf') {
    const database = await initDuckDB();
    const conn = await database.connect();

    // Create tables from Arrow data using CREATE TABLE AS SELECT
    // This is more reliable than insertArrowTable for persistence
    async function createTableFromArrow(table, tableName) {
        // Get column info from schema
        const fields = table.schema.fields;
        const rows = [];
        for (const row of table) {
            const obj = {};
            for (const field of fields) {
                obj[field.name] = row[field.name];
            }
            rows.push(obj);
        }

        if (rows.length === 0) {
            console.warn(`[GAT Bridge] Table ${tableName} has no rows`);
            return;
        }

        // Build CREATE TABLE statement
        const cols = fields.map(f => {
            // Map Arrow types to DuckDB types
            const arrowType = f.type.toString();
            let duckType = 'VARCHAR';
            if (arrowType.includes('Float') || arrowType.includes('Double')) duckType = 'DOUBLE';
            else if (arrowType.includes('Int')) duckType = 'INTEGER';
            return `${f.name} ${duckType}`;
        }).join(', ');

        await conn.query(`DROP TABLE IF EXISTS ${tableName}`);
        await conn.query(`CREATE TABLE ${tableName} (${cols})`);

        // Insert rows using prepared statement
        const placeholders = fields.map(() => '?').join(', ');
        const stmt = await conn.prepare(`INSERT INTO ${tableName} VALUES (${placeholders})`);

        for (const row of rows) {
            const values = fields.map(f => row[f.name]);
            await stmt.query(...values);
        }

        await stmt.close();
    }

    await createTableFromArrow(tables.generators, `${prefix}_generators`);
    await createTableFromArrow(tables.buses, `${prefix}_buses`);
    await createTableFromArrow(tables.branches, `${prefix}_branches`);

    console.log(`[GAT Bridge] Registered tables: ${prefix}_generators, ${prefix}_buses, ${prefix}_branches`);

    return conn;
}

/**
 * Run SQL query on registered OPF results
 * @param {Object} conn - DuckDB connection
 * @param {string} sql - SQL query
 * @returns {Promise<Array<Object>>} Query results as objects
 */
export async function querySql(conn, sql) {
    const result = await conn.query(sql);
    return result.toArray().map(row => row.toJSON());
}

/**
 * Example SQL queries for OPF analysis
 */
export const ExampleQueries = {
    // Top generators by output
    topGenerators: `
        SELECT gen_id, p_mw, q_mvar
        FROM opf_generators
        ORDER BY p_mw DESC
        LIMIT 10
    `,

    // Buses with highest LMPs
    highestLmps: `
        SELECT bus_id, lmp, v_mag
        FROM opf_buses
        WHERE lmp IS NOT NULL
        ORDER BY lmp DESC
        LIMIT 10
    `,

    // Most loaded branches
    loadedBranches: `
        SELECT branch_id, ABS(p_flow_mw) as flow_mw
        FROM opf_branches
        ORDER BY ABS(p_flow_mw) DESC
        LIMIT 10
    `,

    // Summary statistics
    summary: `
        SELECT
            COUNT(*) as n_buses,
            AVG(v_mag) as avg_voltage,
            MIN(v_mag) as min_voltage,
            MAX(v_mag) as max_voltage,
            AVG(lmp) as avg_lmp
        FROM opf_buses
    `,

    // Generation totals
    generationTotals: `
        SELECT
            SUM(p_mw) as total_p_mw,
            SUM(q_mvar) as total_q_mvar,
            COUNT(*) as n_generators
        FROM opf_generators
    `,
};

/**
 * Helper: Format Arrow Table as HTML table
 * @param {Object} table - Arrow Table
 * @param {Object} options - Formatting options
 * @returns {string} HTML table string
 */
export function tableToHtml(table, options = {}) {
    const { maxRows = 100, precision = 4 } = options;
    const fields = table.schema.fields;

    let html = '<table class="gat-results-table">\n<thead>\n<tr>';
    for (const field of fields) {
        html += `<th>${field.name}</th>`;
    }
    html += '</tr>\n</thead>\n<tbody>\n';

    let count = 0;
    for (const row of table) {
        if (count >= maxRows) break;
        html += '<tr>';
        for (const field of fields) {
            const value = row[field.name];
            const formatted = typeof value === 'number'
                ? value.toFixed(precision)
                : value;
            html += `<td>${formatted}</td>`;
        }
        html += '</tr>\n';
        count++;
    }

    html += '</tbody>\n</table>';
    return html;
}

// Export a ready-to-use facade
export default {
    initArrow,
    initDuckDB,
    parseArrowIPC,
    runOpfWithArrow,
    tableToObjects,
    registerInDuckDB,
    querySql,
    tableToHtml,
    ExampleQueries,
};
