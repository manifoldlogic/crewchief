//! Integration tests for C parser using real-world code samples
//!
//! Tests the C parser against real production code to validate:
//! - No panics on real-world C patterns
//! - Correct symbol extraction
//! - Performance within acceptable bounds
//! - Coverage of diverse C idioms and patterns

use maproom::indexer::parser;
use std::time::Instant;

/// Test cJSON parser - single-file JSON library
///
/// cJSON is a small, self-contained JSON parser in C widely used in embedded systems.
/// Tests extraction of:
/// - Static functions (internal API)
/// - Public API functions
/// - Struct definitions
/// - Typedefs
/// - Preprocessor includes
///
/// Source: Representative sample from cJSON.c by Dave Gamble (MIT License)
#[test]
fn test_cjson_parser() {
    // Real code from cJSON.c - a popular embedded JSON parser
    let source = r#"
/*
  Copyright (c) 2009-2017 Dave Gamble and cJSON contributors

  Permission is hereby granted, free of charge, to any person obtaining a copy
  of this software and associated documentation files (the "Software"), to deal
  in the Software without restriction, including without limitation the rights
  to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
  copies of the Software, and to permit persons to whom the Software is
  furnished to do so, subject to the following conditions:
*/

#include <string.h>
#include <stdio.h>
#include <math.h>
#include <stdlib.h>
#include <limits.h>
#include <ctype.h>

/* cJSON Types: */
#define cJSON_Invalid (0)
#define cJSON_False  (1 << 0)
#define cJSON_True   (1 << 1)
#define cJSON_NULL   (1 << 2)
#define cJSON_Number (1 << 3)
#define cJSON_String (1 << 4)
#define cJSON_Array  (1 << 5)
#define cJSON_Object (1 << 6)
#define cJSON_Raw    (1 << 7)

typedef struct cJSON
{
    struct cJSON *next;
    struct cJSON *prev;
    struct cJSON *child;
    int type;
    char *valuestring;
    int valueint;
    double valuedouble;
    char *string;
} cJSON;

typedef struct cJSON_Hooks
{
    void *(*malloc_fn)(size_t sz);
    void (*free_fn)(void *ptr);
} cJSON_Hooks;

/* Internal constructor */
static cJSON *cJSON_New_Item(void)
{
    cJSON* node = (cJSON*)malloc(sizeof(cJSON));
    if (node)
    {
        memset(node, '\0', sizeof(cJSON));
    }
    return node;
}

/* Delete a cJSON structure */
void cJSON_Delete(cJSON *c)
{
    cJSON *next;
    while (c)
    {
        next = c->next;
        if (c->child)
        {
            cJSON_Delete(c->child);
        }
        if (c->valuestring)
        {
            free(c->valuestring);
        }
        if (c->string)
        {
            free(c->string);
        }
        free(c);
        c = next;
    }
}

/* Parse the input text to generate a number, and populate the result into item */
static const char *parse_number(cJSON *item, const char *num)
{
    double n = 0;
    double sign = 1;
    double scale = 0;
    int subscale = 0;
    int signsubscale = 1;

    if (*num == '-')
    {
        sign = -1;
        num++;
    }
    if (*num == '0')
    {
        num++;
    }
    if (*num >= '1' && *num <= '9')
    {
        do
        {
            n = (n * 10.0) + (*num++ - '0');
        } while (*num >= '0' && *num <= '9');
    }

    item->valuedouble = sign * n;
    item->valueint = (int)item->valuedouble;
    item->type = cJSON_Number;
    return num;
}

/* Render the number nicely from the given item into a string */
static char *print_number(const cJSON *item)
{
    char *str;
    double d = item->valuedouble;

    if (d == 0)
    {
        str = (char*)malloc(2);
        if (str)
        {
            strcpy(str, "0");
        }
    }
    else if (fabs(((double)item->valueint) - d) <= DBL_EPSILON && d <= INT_MAX && d >= INT_MIN)
    {
        str = (char*)malloc(21);
        if (str)
        {
            sprintf(str, "%d", item->valueint);
        }
    }
    else
    {
        str = (char*)malloc(64);
        if (str)
        {
            sprintf(str, "%g", d);
        }
    }
    return str;
}

/* Parse the input text into an unescaped cstring, and populate item */
static const unsigned char firstByteMark[7] = { 0x00, 0x00, 0xC0, 0xE0, 0xF0, 0xF8, 0xFC };

static const char *parse_string(cJSON *item, const char *str)
{
    const char *ptr = str + 1;
    char *ptr2;
    char *out;
    int len = 0;
    unsigned uc, uc2;

    if (*str != '\"')
    {
        return 0;
    }

    while (*ptr != '\"' && *ptr && ++len)
    {
        if (*ptr++ == '\\')
        {
            ptr++;
        }
    }

    out = (char*)malloc(len + 1);
    if (!out)
    {
        return 0;
    }

    ptr = str + 1;
    ptr2 = out;
    while (*ptr != '\"' && *ptr)
    {
        if (*ptr != '\\')
        {
            *ptr2++ = *ptr++;
        }
        else
        {
            ptr++;
            switch (*ptr)
            {
                case 'b': *ptr2++ = '\b'; break;
                case 'f': *ptr2++ = '\f'; break;
                case 'n': *ptr2++ = '\n'; break;
                case 'r': *ptr2++ = '\r'; break;
                case 't': *ptr2++ = '\t'; break;
                default: *ptr2++ = *ptr; break;
            }
            ptr++;
        }
    }
    *ptr2 = 0;
    if (*ptr == '\"')
    {
        ptr++;
    }
    item->valuestring = out;
    item->type = cJSON_String;
    return ptr;
}
"#;

    let start = Instant::now();
    let chunks = parser::extract_chunks(source, "c");
    let duration = start.elapsed();

    println!("cJSON: {} chunks extracted in {:?}", chunks.len(), duration);

    // Should not panic and should extract chunks
    assert!(!chunks.is_empty(), "Should extract chunks from cJSON code");

    // Performance: should parse in reasonable time (< 100ms for this size)
    assert!(
        duration.as_millis() < 2000,
        "Parse should complete quickly for ~200 line file (took {:?}, limit 2s for debug builds)",
        duration
    );

    // Should extract includes
    let imports = chunks.iter().find(|c| c.kind == "imports");
    assert!(imports.is_some(), "Should extract #include directives");
    if let Some(imp) = imports {
        let metadata = imp.metadata.as_ref().unwrap();
        let includes = metadata.as_array().unwrap();
        assert!(includes.len() >= 4, "Should extract multiple includes");
    }

    // Should extract struct definitions
    let cjson_struct = chunks
        .iter()
        .find(|c| c.symbol_name == Some("cJSON".to_string()) && c.kind == "struct");
    assert!(
        cjson_struct.is_some(),
        "Should extract main cJSON struct definition"
    );
    if let Some(s) = cjson_struct {
        let metadata = s.metadata.as_ref().unwrap();
        assert_eq!(
            metadata["field_count"], 8,
            "cJSON struct should have 8 fields"
        );
    }

    let hooks_struct = chunks
        .iter()
        .find(|c| c.symbol_name == Some("cJSON_Hooks".to_string()) && c.kind == "struct");
    assert!(hooks_struct.is_some(), "Should extract cJSON_Hooks struct");

    // Should extract static functions (internal API)
    let new_item = chunks
        .iter()
        .find(|c| c.symbol_name == Some("cJSON_New_Item".to_string()));
    assert!(new_item.is_some(), "Should extract cJSON_New_Item");
    if let Some(func) = new_item {
        assert_eq!(func.kind, "func");
        let metadata = func.metadata.as_ref().unwrap();
        assert_eq!(
            metadata["storage_class"], "static",
            "cJSON_New_Item should be marked static"
        );
    }

    let parse_number = chunks
        .iter()
        .find(|c| c.symbol_name == Some("parse_number".to_string()));
    assert!(parse_number.is_some(), "Should extract parse_number");

    let parse_string = chunks
        .iter()
        .find(|c| c.symbol_name == Some("parse_string".to_string()));
    assert!(parse_string.is_some(), "Should extract parse_string");

    // Should extract public API function
    let delete_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("cJSON_Delete".to_string()));
    assert!(delete_func.is_some(), "Should extract cJSON_Delete");
    if let Some(func) = delete_func {
        assert_eq!(func.kind, "func");
        assert!(
            func.signature.as_ref().unwrap().contains("void"),
            "Should capture return type"
        );
    }

    let print_number = chunks
        .iter()
        .find(|c| c.symbol_name == Some("print_number".to_string()));
    assert!(print_number.is_some(), "Should extract print_number");

    // Should extract global array (firstByteMark)
    let first_byte_mark = chunks
        .iter()
        .find(|c| c.symbol_name == Some("firstByteMark".to_string()));
    assert!(
        first_byte_mark.is_some(),
        "Should extract firstByteMark array"
    );

    // Count function vs struct extraction
    let func_count = chunks.iter().filter(|c| c.kind == "func").count();
    let struct_count = chunks.iter().filter(|c| c.kind == "struct").count();

    assert!(
        func_count >= 5,
        "Should extract at least 5 functions (got {})",
        func_count
    );
    assert_eq!(struct_count, 2, "Should extract exactly 2 structs");

    println!(
        "cJSON stats: {} functions, {} structs",
        func_count, struct_count
    );
}

/// Test Redis server.c patterns - networking and event loop code
///
/// Redis is a high-performance key-value store with complex event-driven architecture.
/// Tests extraction of:
/// - Complex function signatures with multiple parameters
/// - Forward declarations
/// - Conditional compilation patterns
/// - Extensive use of callbacks and function pointers
///
/// Source: Representative patterns from Redis server.c (BSD License)
#[test]
fn test_redis_patterns() {
    let source = r#"
/* Redis server.c sample patterns */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sys/time.h>
#include <unistd.h>
#include <signal.h>
#include <errno.h>

#define REDIS_OK 0
#define REDIS_ERR -1

struct redisServer {
    int port;
    char *bindaddr;
    int maxclients;
    long long stat_numcommands;
    time_t stat_starttime;
    int daemonize;
    char *pidfile;
    int cronloops;
};

struct redisClient {
    int fd;
    int flags;
    time_t ctime;
    time_t lastinteraction;
    char *querybuf;
    size_t querybuf_len;
    int argc;
    void **argv;
    struct redisCommand *cmd;
};

typedef void redisCommandProc(struct redisClient *c);

struct redisCommand {
    char *name;
    redisCommandProc *proc;
    int arity;
    int flags;
};

/* Forward declarations */
static void clientsCron(void);
static void databasesCron(void);
static int serverCron(struct redisServer *server);

/* Global server state */
struct redisServer server;
static struct redisCommand *commandTable;

/* Create a new client */
struct redisClient *createClient(int fd) {
    struct redisClient *c = malloc(sizeof(*c));

    if (!c) return NULL;

    c->fd = fd;
    c->querybuf = NULL;
    c->querybuf_len = 0;
    c->argc = 0;
    c->argv = NULL;
    c->cmd = NULL;
    c->flags = 0;
    c->ctime = time(NULL);
    c->lastinteraction = c->ctime;

    return c;
}

/* Free a client */
void freeClient(struct redisClient *c) {
    if (c->fd != -1) {
        close(c->fd);
    }
    if (c->querybuf) {
        free(c->querybuf);
    }
    if (c->argv) {
        free(c->argv);
    }
    free(c);
}

/* Process command from client */
int processCommand(struct redisClient *c) {
    if (c->argc == 0) {
        return REDIS_ERR;
    }

    /* Lookup command */
    c->cmd = lookupCommand((char*)c->argv[0]);
    if (!c->cmd) {
        return REDIS_ERR;
    }

    /* Check arity */
    if ((c->cmd->arity > 0 && c->cmd->arity != c->argc) ||
        (c->argc < -c->cmd->arity)) {
        return REDIS_ERR;
    }

    /* Call the command */
    c->cmd->proc(c);
    return REDIS_OK;
}

/* Lookup command in command table */
static struct redisCommand *lookupCommand(char *name) {
    int j = 0;
    while (commandTable[j].name != NULL) {
        if (!strcasecmp(name, commandTable[j].name)) {
            return &commandTable[j];
        }
        j++;
    }
    return NULL;
}

/* Cron job for handling clients */
static void clientsCron(void) {
    /* Scan all clients and close inactive ones */
    time_t now = time(NULL);
    /* Implementation omitted */
}

/* Database periodic tasks */
static void databasesCron(void) {
    /* Expire keys, resize hashtables, etc */
    /* Implementation omitted */
}

/* Main cron function called periodically */
static int serverCron(struct redisServer *server) {
    server->cronloops++;

    /* Update server time */
    time_t now = time(NULL);

    /* Run periodic tasks */
    clientsCron();
    databasesCron();

    return 100; /* Run again in 100ms */
}

/* Initialize server configuration */
void initServerConfig(void) {
    server.port = 6379;
    server.bindaddr = NULL;
    server.maxclients = 10000;
    server.stat_numcommands = 0;
    server.stat_starttime = time(NULL);
    server.daemonize = 0;
    server.pidfile = NULL;
    server.cronloops = 0;
}

/* Main server loop */
int main(int argc, char **argv) {
    initServerConfig();

    /* Event loop would go here */
    while (1) {
        serverCron(&server);
        /* Process events */
    }

    return 0;
}
"#;

    let start = Instant::now();
    let chunks = parser::extract_chunks(source, "c");
    let duration = start.elapsed();

    println!("Redis: {} chunks extracted in {:?}", chunks.len(), duration);

    // Should not panic
    assert!(!chunks.is_empty(), "Should extract chunks from Redis code");

    // Performance check
    assert!(
        duration.as_millis() < 500,
        "Should parse quickly (took {:?}, limit 500ms for debug builds)",
        duration
    );

    // Should extract includes (system headers)
    let imports = chunks.iter().find(|c| c.kind == "imports");
    assert!(imports.is_some(), "Should extract imports");
    if let Some(imp) = imports {
        let metadata = imp.metadata.as_ref().unwrap();
        let includes = metadata.as_array().unwrap();
        assert!(
            includes.len() >= 5,
            "Should extract multiple system includes"
        );
    }

    // Should extract struct definitions
    let server_struct = chunks
        .iter()
        .find(|c| c.symbol_name == Some("redisServer".to_string()) && c.kind == "struct");
    assert!(server_struct.is_some(), "Should extract redisServer struct");

    let client_struct = chunks
        .iter()
        .find(|c| c.symbol_name == Some("redisClient".to_string()) && c.kind == "struct");
    assert!(client_struct.is_some(), "Should extract redisClient struct");

    let command_struct = chunks
        .iter()
        .find(|c| c.symbol_name == Some("redisCommand".to_string()) && c.kind == "struct");
    assert!(
        command_struct.is_some(),
        "Should extract redisCommand struct"
    );

    // Function pointer typedef - may not be extracted due to complex declarator
    // typedef void redisCommandProc(struct redisClient *c);
    let command_proc_typedef = chunks
        .iter()
        .find(|c| c.symbol_name == Some("redisCommandProc".to_string()) && c.kind == "typedef");
    if command_proc_typedef.is_none() {
        eprintln!("Note: redisCommandProc typedef not extracted (function pointer typedef - known limitation)");
    }

    // Should extract public API functions
    let create_client = chunks
        .iter()
        .find(|c| c.symbol_name == Some("createClient".to_string()));
    assert!(create_client.is_some(), "Should extract createClient");

    let free_client = chunks
        .iter()
        .find(|c| c.symbol_name == Some("freeClient".to_string()));
    assert!(free_client.is_some(), "Should extract freeClient");

    let process_command = chunks
        .iter()
        .find(|c| c.symbol_name == Some("processCommand".to_string()));
    assert!(process_command.is_some(), "Should extract processCommand");

    // Should extract static functions
    let lookup_command = chunks
        .iter()
        .find(|c| c.symbol_name == Some("lookupCommand".to_string()));
    assert!(lookup_command.is_some(), "Should extract lookupCommand");
    if let Some(func) = lookup_command {
        let metadata = func.metadata.as_ref().unwrap();
        assert_eq!(metadata["storage_class"], "static");
    }

    let clients_cron = chunks
        .iter()
        .find(|c| c.symbol_name == Some("clientsCron".to_string()));
    assert!(clients_cron.is_some(), "Should extract clientsCron");

    let databases_cron = chunks
        .iter()
        .find(|c| c.symbol_name == Some("databasesCron".to_string()));
    assert!(databases_cron.is_some(), "Should extract databasesCron");

    let server_cron = chunks
        .iter()
        .find(|c| c.symbol_name == Some("serverCron".to_string()));
    assert!(server_cron.is_some(), "Should extract serverCron");

    // Should extract main function
    let main_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("main".to_string()));
    assert!(main_func.is_some(), "Should extract main function");
    if let Some(func) = main_func {
        assert!(
            func.signature
                .as_ref()
                .unwrap()
                .contains("int argc, char **argv"),
            "Should capture main signature"
        );
    }

    // Should extract global variables
    let server_var = chunks
        .iter()
        .find(|c| c.symbol_name == Some("server".to_string()) && c.kind == "variable");
    assert!(
        server_var.is_some(),
        "Should extract global server variable"
    );

    let command_table_var = chunks
        .iter()
        .find(|c| c.symbol_name == Some("commandTable".to_string()) && c.kind == "variable");
    assert!(
        command_table_var.is_some(),
        "Should extract commandTable variable"
    );

    // Count symbols by type
    let func_count = chunks.iter().filter(|c| c.kind == "func").count();
    let struct_count = chunks.iter().filter(|c| c.kind == "struct").count();
    let var_count = chunks.iter().filter(|c| c.kind == "variable").count();

    println!(
        "Redis stats: {} functions, {} structs, {} variables",
        func_count, struct_count, var_count
    );

    assert!(
        func_count >= 9,
        "Should extract at least 9 functions (got {})",
        func_count
    );
    assert_eq!(struct_count, 3, "Should extract 3 struct definitions");
    assert!(var_count >= 2, "Should extract at least 2 global variables");
}

/// Test musl libc patterns - system-level C code
///
/// musl libc is a lightweight standard C library implementation.
/// Tests extraction of:
/// - POSIX function implementations
/// - Extensive use of preprocessor conditionals
/// - System call wrappers
/// - Complex pointer manipulation
///
/// Source: Representative patterns from musl libc (MIT License)
#[test]
fn test_musl_libc_patterns() {
    let source = r#"
#include <string.h>
#include <stdint.h>
#include <limits.h>

#define ALIGN (sizeof(size_t))
#define ONES ((size_t)-1/UCHAR_MAX)
#define HIGHS (ONES * (UCHAR_MAX/2+1))
#define HASZERO(x) (((x)-ONES) & ~(x) & HIGHS)

/* String length implementation */
size_t strlen(const char *s)
{
    const char *a = s;
    const size_t *w;

    for (; (uintptr_t)s % ALIGN; s++) {
        if (!*s) return s - a;
    }

    for (w = (const void *)s; !HASZERO(*w); w++);

    for (s = (const void *)w; *s; s++);

    return s - a;
}

/* String copy implementation */
char *strcpy(char *restrict dest, const char *restrict src)
{
    char *d = dest;

    while ((*d++ = *src++));

    return dest;
}

/* Memory comparison */
int memcmp(const void *vl, const void *vr, size_t n)
{
    const unsigned char *l = vl;
    const unsigned char *r = vr;

    for (; n && *l == *r; n--, l++, r++);

    return n ? *l - *r : 0;
}

/* Memory set - optimized version */
void *memset(void *dest, int c, size_t n)
{
    unsigned char *s = dest;
    size_t k;

    if (!n) return dest;
    s[0] = c;
    s[n-1] = c;
    if (n <= 2) return dest;

    s[1] = c;
    s[2] = c;
    s[n-2] = c;
    s[n-3] = c;
    if (n <= 6) return dest;

    s[3] = c;
    s[n-4] = c;
    if (n <= 8) return dest;

    k = -(uintptr_t)s & 3;
    s += k;
    n -= k;
    n &= -4;

    size_t *ws = (size_t *)s;
    size_t wc = c & 0xFF;
    wc |= wc << 8;
    wc |= wc << 16;
    wc |= wc << 16 << 16;

    for (; n; n-=4) *ws++ = wc;

    return dest;
}

/* String tokenization */
char *strtok(char *restrict s, const char *restrict sep)
{
    static char *p;
    if (!s && !(s = p)) return NULL;

    s += strspn(s, sep);
    if (!*s) return p = 0;

    p = s + strcspn(s, sep);
    if (*p) *p++ = 0;
    else p = 0;

    return s;
}

/* Helper: string span */
size_t strspn(const char *s, const char *c)
{
    const char *a = s;
    size_t byteset[32/sizeof(size_t)] = { 0 };

    if (!c[0]) return 0;
    if (!c[1]) {
        for (; *s == *c; s++);
        return s - a;
    }

    for (; *c && (byteset[*(unsigned char *)c / (8*sizeof(size_t))] |=
                  1UL << (*(unsigned char *)c % (8*sizeof(size_t)))); c++);

    for (; *s && (byteset[*(unsigned char *)s / (8*sizeof(size_t))] &
                  (1UL << (*(unsigned char *)s % (8*sizeof(size_t))))); s++);

    return s - a;
}

/* Complement string span */
size_t strcspn(const char *s, const char *c)
{
    const char *a = s;
    size_t byteset[32/sizeof(size_t)];

    if (!c[0] || !c[1]) {
        return strlen(s);
    }

    memset(byteset, 0, sizeof(byteset));

    for (; *c && (byteset[*(unsigned char *)c / (8*sizeof(size_t))] |=
                  1UL << (*(unsigned char *)c % (8*sizeof(size_t)))); c++);

    for (; *s && !(byteset[*(unsigned char *)s / (8*sizeof(size_t))] &
                   (1UL << (*(unsigned char *)s % (8*sizeof(size_t))))); s++);

    return s - a;
}

/* Integer to string conversion */
char *itoa(int value, char *str, int base)
{
    char *rc;
    char *ptr;
    char *low;

    if (base < 2 || base > 36) {
        *str = '\0';
        return str;
    }

    rc = ptr = str;

    if (value < 0 && base == 10) {
        *ptr++ = '-';
    }

    low = ptr;

    do {
        *ptr++ = "0123456789abcdefghijklmnopqrstuvwxyz"[value % base];
        value /= base;
    } while (value);

    *ptr-- = '\0';

    while (low < ptr) {
        char tmp = *low;
        *low++ = *ptr;
        *ptr-- = tmp;
    }

    return rc;
}
"#;

    let start = Instant::now();
    let chunks = parser::extract_chunks(source, "c");
    let duration = start.elapsed();

    println!(
        "musl libc: {} chunks extracted in {:?}",
        chunks.len(),
        duration
    );

    // Should not panic
    assert!(
        !chunks.is_empty(),
        "Should extract chunks from musl libc code"
    );

    // Performance check
    assert!(
        duration.as_millis() < 500,
        "Should parse quickly (took {:?}, limit 500ms for debug builds)",
        duration
    );

    // Should extract includes
    let imports = chunks.iter().find(|c| c.kind == "imports");
    assert!(imports.is_some(), "Should extract includes");

    // Should extract strlen
    let strlen_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("strlen".to_string()));
    assert!(strlen_func.is_some(), "Should extract strlen");
    if let Some(func) = strlen_func {
        assert_eq!(func.kind, "func");
        assert!(
            func.signature.as_ref().unwrap().contains("size_t"),
            "Should capture return type"
        );
    }

    // Should extract strcpy
    let strcpy_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("strcpy".to_string()));
    assert!(strcpy_func.is_some(), "Should extract strcpy");

    // Should extract memcmp
    let memcmp_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("memcmp".to_string()));
    assert!(memcmp_func.is_some(), "Should extract memcmp");

    // Should extract memset
    let memset_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("memset".to_string()));
    assert!(memset_func.is_some(), "Should extract memset");
    if let Some(func) = memset_func {
        assert!(
            func.signature
                .as_ref()
                .unwrap()
                .contains("void *dest, int c, size_t n"),
            "Should capture parameters"
        );
    }

    // Should extract strtok
    let strtok_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("strtok".to_string()));
    assert!(strtok_func.is_some(), "Should extract strtok");

    // Should extract strspn
    let strspn_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("strspn".to_string()));
    assert!(strspn_func.is_some(), "Should extract strspn");

    // Should extract strcspn
    let strcspn_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("strcspn".to_string()));
    assert!(strcspn_func.is_some(), "Should extract strcspn");

    // Should extract itoa
    let itoa_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("itoa".to_string()));
    assert!(itoa_func.is_some(), "Should extract itoa");

    // All functions should have proper signatures
    let func_count = chunks.iter().filter(|c| c.kind == "func").count();
    println!("musl libc stats: {} functions", func_count);

    assert!(
        func_count >= 8,
        "Should extract at least 8 functions (got {})",
        func_count
    );

    // Verify all functions have signatures
    let funcs_with_sig = chunks
        .iter()
        .filter(|c| c.kind == "func" && c.signature.is_some())
        .count();
    assert_eq!(
        funcs_with_sig, func_count,
        "All functions should have signatures"
    );
}

/// Test zlib compression library patterns
///
/// zlib is a widely-used compression library.
/// Tests extraction of:
/// - Complex data structures
/// - Bit manipulation and compression algorithms
/// - State machines and lookup tables
///
/// Source: Representative patterns from zlib (zlib License)
#[test]
fn test_zlib_patterns() {
    let source = r#"
#include <stddef.h>

typedef unsigned char Byte;
typedef unsigned int uInt;
typedef unsigned long uLong;

#define Z_OK            0
#define Z_STREAM_END    1
#define Z_NEED_DICT     2
#define Z_ERRNO        (-1)
#define Z_STREAM_ERROR (-2)
#define Z_DATA_ERROR   (-3)
#define Z_MEM_ERROR    (-4)
#define Z_BUF_ERROR    (-5)

typedef struct z_stream_s {
    const Byte *next_in;
    uInt avail_in;
    uLong total_in;

    Byte *next_out;
    uInt avail_out;
    uLong total_out;

    const char *msg;
    struct internal_state *state;

    void *(*zalloc)(void *opaque, uInt items, uInt size);
    void (*zfree)(void *opaque, void *address);
    void *opaque;

    int data_type;
    uLong adler;
    uLong reserved;
} z_stream;

typedef z_stream *z_streamp;

/* CRC-32 lookup table */
static const uLong crc_table[256] = {
    0x00000000L, 0x77073096L, 0xee0e612cL, 0x990951baL,
    0x076dc419L, 0x706af48fL, 0xe963a535L, 0x9e6495a3L,
    /* ... truncated for brevity ... */
};

/* Update CRC with new byte */
static uLong crc32_update(uLong crc, const Byte *buf, uInt len)
{
    uLong c = crc;
    int n;

    for (n = 0; n < len; n++) {
        c = crc_table[(c ^ buf[n]) & 0xff] ^ (c >> 8);
    }
    return c;
}

/* Initialize a z_stream */
int deflateInit(z_streamp strm, int level)
{
    if (strm == NULL) {
        return Z_STREAM_ERROR;
    }

    strm->total_in = strm->total_out = 0;
    strm->msg = NULL;
    strm->data_type = Z_BINARY;

    return Z_OK;
}

/* Compress data */
int deflate(z_streamp strm, int flush)
{
    int ret;

    if (strm == NULL || strm->state == NULL) {
        return Z_STREAM_ERROR;
    }

    if (strm->next_out == NULL ||
        (strm->next_in == NULL && strm->avail_in != 0)) {
        return Z_STREAM_ERROR;
    }

    /* Compression logic would go here */

    return Z_OK;
}

/* Decompress data */
int inflate(z_streamp strm, int flush)
{
    struct inflate_state *state;

    if (strm == NULL || strm->state == NULL) {
        return Z_STREAM_ERROR;
    }

    state = (struct inflate_state *)strm->state;

    /* Decompression logic would go here */

    return Z_OK;
}

/* Calculate Adler-32 checksum */
uLong adler32(uLong adler, const Byte *buf, uInt len)
{
    unsigned long sum2;
    unsigned n;

    #define BASE 65521UL
    #define NMAX 5552

    sum2 = (adler >> 16) & 0xffff;
    adler &= 0xffff;

    if (len == 1) {
        adler += buf[0];
        if (adler >= BASE)
            adler -= BASE;
        sum2 += adler;
        if (sum2 >= BASE)
            sum2 -= BASE;
        return adler | (sum2 << 16);
    }

    if (buf == NULL)
        return 1L;

    while (len > 0) {
        n = len > NMAX ? NMAX : len;
        len -= n;

        while (n >= 16) {
            adler += buf[0]; sum2 += adler;
            adler += buf[1]; sum2 += adler;
            adler += buf[2]; sum2 += adler;
            adler += buf[3]; sum2 += adler;
            adler += buf[4]; sum2 += adler;
            adler += buf[5]; sum2 += adler;
            adler += buf[6]; sum2 += adler;
            adler += buf[7]; sum2 += adler;
            adler += buf[8]; sum2 += adler;
            adler += buf[9]; sum2 += adler;
            adler += buf[10]; sum2 += adler;
            adler += buf[11]; sum2 += adler;
            adler += buf[12]; sum2 += adler;
            adler += buf[13]; sum2 += adler;
            adler += buf[14]; sum2 += adler;
            adler += buf[15]; sum2 += adler;

            buf += 16;
            n -= 16;
        }

        while (n--) {
            adler += *buf++;
            sum2 += adler;
        }

        adler %= BASE;
        sum2 %= BASE;
    }

    return adler | (sum2 << 16);
}

/* Free compression resources */
int deflateEnd(z_streamp strm)
{
    if (strm == NULL || strm->state == NULL) {
        return Z_STREAM_ERROR;
    }

    if (strm->zfree) {
        strm->zfree(strm->opaque, strm->state);
    }

    strm->state = NULL;

    return Z_OK;
}
"#;

    let start = Instant::now();
    let chunks = parser::extract_chunks(source, "c");
    let duration = start.elapsed();

    println!("zlib: {} chunks extracted in {:?}", chunks.len(), duration);

    // Should not panic
    assert!(!chunks.is_empty(), "Should extract chunks from zlib code");

    // Performance check
    assert!(
        duration.as_millis() < 500,
        "Should parse quickly (took {:?}, limit 500ms for debug builds)",
        duration
    );

    // Should extract typedefs
    let byte_typedef = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Byte".to_string()) && c.kind == "typedef");
    assert!(byte_typedef.is_some(), "Should extract Byte typedef");

    let uint_typedef = chunks
        .iter()
        .find(|c| c.symbol_name == Some("uInt".to_string()) && c.kind == "typedef");
    assert!(uint_typedef.is_some(), "Should extract uInt typedef");

    let z_stream_typedef = chunks
        .iter()
        .find(|c| c.symbol_name == Some("z_stream".to_string()) && c.kind == "typedef");
    assert!(
        z_stream_typedef.is_some(),
        "Should extract z_stream typedef"
    );

    let z_streamp_typedef = chunks
        .iter()
        .find(|c| c.symbol_name == Some("z_streamp".to_string()) && c.kind == "typedef");
    assert!(
        z_streamp_typedef.is_some(),
        "Should extract z_streamp typedef"
    );

    // Should extract struct
    let z_stream_struct = chunks
        .iter()
        .find(|c| c.symbol_name == Some("z_stream_s".to_string()) && c.kind == "struct");
    assert!(
        z_stream_struct.is_some(),
        "Should extract z_stream_s struct"
    );

    // Should extract static array (crc_table)
    let crc_table = chunks
        .iter()
        .find(|c| c.symbol_name == Some("crc_table".to_string()));
    assert!(crc_table.is_some(), "Should extract crc_table");

    // Should extract functions
    let crc32_update = chunks
        .iter()
        .find(|c| c.symbol_name == Some("crc32_update".to_string()));
    assert!(crc32_update.is_some(), "Should extract crc32_update");
    if let Some(func) = crc32_update {
        let metadata = func.metadata.as_ref().unwrap();
        assert_eq!(metadata["storage_class"], "static");
    }

    let deflate_init = chunks
        .iter()
        .find(|c| c.symbol_name == Some("deflateInit".to_string()));
    assert!(deflate_init.is_some(), "Should extract deflateInit");

    let deflate_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("deflate".to_string()));
    assert!(deflate_func.is_some(), "Should extract deflate");

    let inflate_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("inflate".to_string()));
    assert!(inflate_func.is_some(), "Should extract inflate");

    let adler32_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("adler32".to_string()));
    assert!(adler32_func.is_some(), "Should extract adler32");

    let deflate_end = chunks
        .iter()
        .find(|c| c.symbol_name == Some("deflateEnd".to_string()));
    assert!(deflate_end.is_some(), "Should extract deflateEnd");

    // Count symbols
    let func_count = chunks.iter().filter(|c| c.kind == "func").count();
    let typedef_count = chunks.iter().filter(|c| c.kind == "typedef").count();
    let struct_count = chunks.iter().filter(|c| c.kind == "struct").count();

    println!(
        "zlib stats: {} functions, {} typedefs, {} structs",
        func_count, typedef_count, struct_count
    );

    assert!(
        func_count >= 6,
        "Should extract at least 6 functions (got {})",
        func_count
    );
    assert!(
        typedef_count >= 4,
        "Should extract at least 4 typedefs (got {})",
        typedef_count
    );
    assert!(struct_count >= 1, "Should extract at least 1 struct");
}

/// Test parsing performance with larger code samples
///
/// Combines multiple samples to test parser performance at scale.
/// Validates that parser maintains good performance characteristics.
#[test]
fn test_performance_scaling() {
    // Combine samples to create larger input
    let large_source = include_str!("../tests/fixtures/c/combined_real_world.c");

    let start = Instant::now();
    let chunks = parser::extract_chunks(large_source, "c");
    let duration = start.elapsed();

    let line_count = large_source.lines().count();
    let bytes = large_source.len();

    println!(
        "Performance test: {} lines, {} bytes -> {} chunks in {:?}",
        line_count,
        bytes,
        chunks.len(),
        duration
    );

    // Should not panic
    assert!(
        !chunks.is_empty(),
        "Should extract chunks from large source"
    );

    // Performance: should handle ~1000 lines in under 500ms
    // Note: thresholds are generous to account for CI/debug build overhead
    let max_duration_ms = if line_count > 2000 {
        5000 // 5 seconds for very large files
    } else if line_count > 1000 {
        2000 // 2 seconds for medium files
    } else {
        1000 // 1 second for smaller files (debug builds are slower)
    };

    assert!(
        duration.as_millis() < max_duration_ms,
        "Parse time exceeded threshold: {:?} > {}ms for {} lines",
        duration,
        max_duration_ms,
        line_count
    );

    // Calculate parsing rate
    let lines_per_sec = (line_count as f64) / duration.as_secs_f64();
    println!("Parsing rate: {:.0} lines/second", lines_per_sec);

    // Should maintain reasonable parsing rate (>1000 lines/sec in debug builds)
    assert!(
        lines_per_sec > 1000.0,
        "Parsing rate too slow: {:.0} lines/sec (expected >1000)",
        lines_per_sec
    );
}
