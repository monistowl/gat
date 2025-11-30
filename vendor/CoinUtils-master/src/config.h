/* src/config.h.  Generated from config.h.in by configure.  */
/* src/config.h.in.  Generated from configure.ac by autoheader.  */

/* Library Visibility Attribute */
#define COINUTILSLIB_EXPORT 

/* Library Visibility Attribute */
#define COINUTILSTEST_EXPORT 

/* Define to 1 if CoinBigIndex is int */
#define COINUTILS_BIGINDEX_IS_INT 1

/* Define to type of CoinBigIndex */
#define COINUTILS_BIGINDEX_T int

/* Define to the debug sanity check level (0 is no test) */
#define COINUTILS_CHECKLEVEL 0

/* Define to 1 if CoinUtils uses C++11 */
#define COINUTILS_CPLUSPLUS11 1

/* Define to be the name of C-function for Inf check */
#define COINUTILS_C_FINITE std::isfinite

/* Define to be the name of C-function for NaN check */
#define COINUTILS_C_ISNAN std::isnan

/* Define to 1 if ASL is available. */
/* #undef COINUTILS_HAS_ASL */

/* Define to 1 if bzlib is available */
#define COINUTILS_HAS_BZLIB 1

/* Define to 1 if cstdint is available for CoinUtils */
#define COINUTILS_HAS_CSTDINT 1

/* Define to 1 if Glpk is available. */
/* #undef COINUTILS_HAS_GLPK */

/* Define to 1 if the LAPACK package is available */
#define COINUTILS_HAS_LAPACK 1

/* Define to 1 if Netlib is available. */
/* #undef COINUTILS_HAS_NETLIB */

/* Define to 1 if readline is available */
/* #undef COINUTILS_HAS_READLINE */

/* Define to 1 if Sample is available. */
#define COINUTILS_HAS_SAMPLE 1

/* Define to 1 if stdint.h is available for CoinUtils */
#define COINUTILS_HAS_STDINT_H 1

/* Define to 1 if zlib is available */
#define COINUTILS_HAS_ZLIB 1

/* Define to 64-bit integer type */
#define COINUTILS_INT64_T int64_t

/* Define to integer type capturing pointer */
#define COINUTILS_INTPTR_T intptr_t

/* Define to a macro mangling the given C identifier (in lower and upper
   case). */
#define COINUTILS_LAPACK_FUNC(name,NAME) name ## _

/* As COINUTILS_LAPACK_FUNC, but for C identifiers containing underscores. */
#define COINUTILS_LAPACK_FUNC_(name,NAME) name ## _

/* Default maximum size allocated from pool. */
#define COINUTILS_MEMPOOL_MAXPOOLED -1

/* Define to 1 if CoinUtils should override global new/delete. */
/* #undef COINUTILS_MEMPOOL_OVERRIDE_NEW */

/* Define to 1 if the thread aware version of CoinUtils should be compiled */
/* #undef COINUTILS_PTHREADS */

/* Define to 64-bit unsigned integer type */
#define COINUTILS_UINT64_T uint64_t

/* Define to the debug verbosity level (0 is no output) */
#define COINUTILS_VERBOSITY 0

/* Version number of project */
#define COINUTILS_VERSION "devel"

/* Major version number of project. */
#define COINUTILS_VERSION_MAJOR 9999

/* Minor version number of project. */
#define COINUTILS_VERSION_MINOR 9999

/* Release version number of project. */
#define COINUTILS_VERSION_RELEASE 9999

/* Define to 1 if your C++ compiler doesn't accept -c and -o together. */
/* #undef CXX_NO_MINUS_C_MINUS_O */

/* Define to 1 if you have the <cfloat> header file. */
#define HAVE_CFLOAT 1

/* Define to 1 if you have the <cieeefp> header file. */
/* #undef HAVE_CIEEEFP */

/* Define to 1 if you have the <cmath> header file. */
#define HAVE_CMATH 1

/* Define to 1 if you have the <cstdint> header file. */
#define HAVE_CSTDINT 1

/* Define to 1 if you have the <dlfcn.h> header file. */
#define HAVE_DLFCN_H 1

/* Define to 1 if you have the <endian.h> header file. */
#define HAVE_ENDIAN_H 1

/* Define to 1 if you have the <execinfo.h> header file. */
#define HAVE_EXECINFO_H 1

/* Define to 1 if you have the <float.h> header file. */
/* #undef HAVE_FLOAT_H */

/* Define to 1 if you have the <ieeefp.h> header file. */
/* #undef HAVE_IEEEFP_H */

/* Define to 1 if you have the <inttypes.h> header file. */
#define HAVE_INTTYPES_H 1

/* Define to 1 if you have the 'bz2' library (-lbz2). */
#define HAVE_LIBBZ2 1

/* Define to 1 if you have the 'readline' library (-lreadline). */
/* #undef HAVE_LIBREADLINE */

/* Define to 1 if you have the 'z' library (-lz). */
#define HAVE_LIBZ 1

/* Define to 1 if you have the <math.h> header file. */
/* #undef HAVE_MATH_H */

/* Define to 1 if you have the <stdint.h> header file. */
#define HAVE_STDINT_H 1

/* Define to 1 if you have the <stdio.h> header file. */
#define HAVE_STDIO_H 1

/* Define to 1 if you have the <stdlib.h> header file. */
#define HAVE_STDLIB_H 1

/* Define to 1 if you have the <strings.h> header file. */
#define HAVE_STRINGS_H 1

/* Define to 1 if you have the <string.h> header file. */
#define HAVE_STRING_H 1

/* Define to 1 if you have the <sys/stat.h> header file. */
#define HAVE_SYS_STAT_H 1

/* Define to 1 if you have the <sys/types.h> header file. */
#define HAVE_SYS_TYPES_H 1

/* Define to 1 if you have the <unistd.h> header file. */
#define HAVE_UNISTD_H 1

/* Define to 1 if you have the <windows.h> header file. */
/* #undef HAVE_WINDOWS_H */

/* Define to the sub-directory where libtool stores uninstalled libraries. */
#define LT_OBJDIR ".libs/"

/* Define to the address where bug reports for this package should be sent. */
#define PACKAGE_BUGREPORT "https://github.com/coin-or/CoinUtils/issues/new"

/* Define to the full name of this package. */
#define PACKAGE_NAME "CoinUtils"

/* Define to the full name and version of this package. */
#define PACKAGE_STRING "CoinUtils devel"

/* Define to the one symbol short name of this package. */
#define PACKAGE_TARNAME "coin-or-coinutils"

/* Define to the home page for this package. */
#define PACKAGE_URL "https://github.com/coin-or/CoinUtils"

/* Define to the version of this package. */
#define PACKAGE_VERSION "devel"

/* The size of 'int', as computed by sizeof. */
/* #undef SIZEOF_INT */

/* The size of 'int *', as computed by sizeof. */
/* #undef SIZEOF_INT_P */

/* The size of 'long', as computed by sizeof. */
/* #undef SIZEOF_LONG */

/* The size of 'long long', as computed by sizeof. */
/* #undef SIZEOF_LONG_LONG */

/* Define to 1 if all of the C89 standard headers exist (not just the ones
   required in a freestanding environment). This macro is provided for
   backward compatibility; new code need not use it. */
#define STDC_HEADERS 1
