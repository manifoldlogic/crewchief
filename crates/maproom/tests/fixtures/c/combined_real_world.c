/*
 * Combined C code samples from real-world projects for performance testing
 *
 * This file combines representative samples from:
 * - cJSON (MIT License) - JSON parser
 * - Redis (BSD License) - Key-value store
 * - musl libc (MIT License) - C standard library
 * - zlib (zlib License) - Compression library
 *
 * Purpose: Test parser performance at scale with realistic C patterns
 */

/* ========== Part 1: cJSON patterns ========== */

#include <string.h>
#include <stdio.h>
#include <math.h>
#include <stdlib.h>
#include <limits.h>
#include <ctype.h>

/* cJSON Types */
#define cJSON_Invalid (0)
#define cJSON_False  (1 << 0)
#define cJSON_True   (1 << 1)
#define cJSON_NULL   (1 << 2)
#define cJSON_Number (1 << 3)
#define cJSON_String (1 << 4)
#define cJSON_Array  (1 << 5)
#define cJSON_Object (1 << 6)

typedef struct cJSON {
    struct cJSON *next;
    struct cJSON *prev;
    struct cJSON *child;
    int type;
    char *valuestring;
    int valueint;
    double valuedouble;
    char *string;
} cJSON;

typedef struct cJSON_Hooks {
    void *(*malloc_fn)(size_t sz);
    void (*free_fn)(void *ptr);
} cJSON_Hooks;

static cJSON *cJSON_New_Item(void) {
    cJSON* node = (cJSON*)malloc(sizeof(cJSON));
    if (node) {
        memset(node, '\0', sizeof(cJSON));
    }
    return node;
}

void cJSON_Delete(cJSON *c) {
    cJSON *next;
    while (c) {
        next = c->next;
        if (c->child) {
            cJSON_Delete(c->child);
        }
        if (c->valuestring) {
            free(c->valuestring);
        }
        if (c->string) {
            free(c->string);
        }
        free(c);
        c = next;
    }
}

static const char *parse_number(cJSON *item, const char *num) {
    double n = 0;
    double sign = 1;

    if (*num == '-') {
        sign = -1;
        num++;
    }
    if (*num == '0') {
        num++;
    }
    if (*num >= '1' && *num <= '9') {
        do {
            n = (n * 10.0) + (*num++ - '0');
        } while (*num >= '0' && *num <= '9');
    }

    item->valuedouble = sign * n;
    item->valueint = (int)item->valuedouble;
    item->type = cJSON_Number;
    return num;
}

/* ========== Part 2: Redis patterns ========== */

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

static void clientsCron(void);
static void databasesCron(void);
static int serverCron(struct redisServer *server);

struct redisServer server;
static struct redisCommand *commandTable;

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

int processCommand(struct redisClient *c) {
    if (c->argc == 0) {
        return REDIS_ERR;
    }

    c->cmd = lookupCommand((char*)c->argv[0]);
    if (!c->cmd) {
        return REDIS_ERR;
    }

    if ((c->cmd->arity > 0 && c->cmd->arity != c->argc) ||
        (c->argc < -c->cmd->arity)) {
        return REDIS_ERR;
    }

    c->cmd->proc(c);
    return REDIS_OK;
}

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

static void clientsCron(void) {
    time_t now = time(NULL);
    /* Implementation omitted */
}

static void databasesCron(void) {
    /* Implementation omitted */
}

static int serverCron(struct redisServer *server) {
    server->cronloops++;
    time_t now = time(NULL);

    clientsCron();
    databasesCron();

    return 100;
}

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

/* ========== Part 3: musl libc patterns ========== */

#include <stdint.h>

#define ALIGN (sizeof(size_t))
#define ONES ((size_t)-1/UCHAR_MAX)
#define HIGHS (ONES * (UCHAR_MAX/2+1))
#define HASZERO(x) (((x)-ONES) & ~(x) & HIGHS)

size_t strlen(const char *s) {
    const char *a = s;
    const size_t *w;

    for (; (uintptr_t)s % ALIGN; s++) {
        if (!*s) return s - a;
    }

    for (w = (const void *)s; !HASZERO(*w); w++);
    for (s = (const void *)w; *s; s++);

    return s - a;
}

char *strcpy(char *restrict dest, const char *restrict src) {
    char *d = dest;
    while ((*d++ = *src++));
    return dest;
}

int memcmp(const void *vl, const void *vr, size_t n) {
    const unsigned char *l = vl;
    const unsigned char *r = vr;

    for (; n && *l == *r; n--, l++, r++);
    return n ? *l - *r : 0;
}

void *memset(void *dest, int c, size_t n) {
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

char *strtok(char *restrict s, const char *restrict sep) {
    static char *p;
    if (!s && !(s = p)) return NULL;

    s += strspn(s, sep);
    if (!*s) return p = 0;

    p = s + strcspn(s, sep);
    if (*p) *p++ = 0;
    else p = 0;

    return s;
}

size_t strspn(const char *s, const char *c) {
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

size_t strcspn(const char *s, const char *c) {
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

/* ========== Part 4: zlib patterns ========== */

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

static const uLong crc_table[256] = {
    0x00000000L, 0x77073096L, 0xee0e612cL, 0x990951baL,
    0x076dc419L, 0x706af48fL, 0xe963a535L, 0x9e6495a3L,
    0x0edb8832L, 0x79dcb8a4L, 0xe0d5e91eL, 0x97d2d988L
};

static uLong crc32_update(uLong crc, const Byte *buf, uInt len) {
    uLong c = crc;
    int n;

    for (n = 0; n < len; n++) {
        c = crc_table[(c ^ buf[n]) & 0xff] ^ (c >> 8);
    }
    return c;
}

int deflateInit(z_streamp strm, int level) {
    if (strm == NULL) {
        return Z_STREAM_ERROR;
    }

    strm->total_in = strm->total_out = 0;
    strm->msg = NULL;
    strm->data_type = Z_BINARY;

    return Z_OK;
}

int deflate(z_streamp strm, int flush) {
    if (strm == NULL || strm->state == NULL) {
        return Z_STREAM_ERROR;
    }

    if (strm->next_out == NULL ||
        (strm->next_in == NULL && strm->avail_in != 0)) {
        return Z_STREAM_ERROR;
    }

    return Z_OK;
}

int inflate(z_streamp strm, int flush) {
    struct inflate_state *state;

    if (strm == NULL || strm->state == NULL) {
        return Z_STREAM_ERROR;
    }

    state = (struct inflate_state *)strm->state;
    return Z_OK;
}

uLong adler32(uLong adler, const Byte *buf, uInt len) {
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
            buf += 4;
            n -= 4;
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

int deflateEnd(z_streamp strm) {
    if (strm == NULL || strm->state == NULL) {
        return Z_STREAM_ERROR;
    }

    if (strm->zfree) {
        strm->zfree(strm->opaque, strm->state);
    }

    strm->state = NULL;
    return Z_OK;
}

/* ========== Additional utility functions for scale ========== */

/* Simple hash table implementation */
#define HASH_SIZE 1024

struct hash_entry {
    char *key;
    void *value;
    struct hash_entry *next;
};

struct hash_table {
    struct hash_entry *buckets[HASH_SIZE];
    size_t count;
};

static unsigned int hash_string(const char *str) {
    unsigned int hash = 5381;
    int c;

    while ((c = *str++)) {
        hash = ((hash << 5) + hash) + c;
    }

    return hash % HASH_SIZE;
}

struct hash_table *hash_create(void) {
    struct hash_table *ht = malloc(sizeof(struct hash_table));
    if (!ht) return NULL;

    memset(ht->buckets, 0, sizeof(ht->buckets));
    ht->count = 0;

    return ht;
}

int hash_insert(struct hash_table *ht, const char *key, void *value) {
    unsigned int idx = hash_string(key);
    struct hash_entry *entry = malloc(sizeof(struct hash_entry));

    if (!entry) return -1;

    entry->key = strdup(key);
    entry->value = value;
    entry->next = ht->buckets[idx];
    ht->buckets[idx] = entry;
    ht->count++;

    return 0;
}

void *hash_lookup(struct hash_table *ht, const char *key) {
    unsigned int idx = hash_string(key);
    struct hash_entry *entry = ht->buckets[idx];

    while (entry) {
        if (strcmp(entry->key, key) == 0) {
            return entry->value;
        }
        entry = entry->next;
    }

    return NULL;
}

void hash_destroy(struct hash_table *ht) {
    size_t i;

    for (i = 0; i < HASH_SIZE; i++) {
        struct hash_entry *entry = ht->buckets[i];
        while (entry) {
            struct hash_entry *next = entry->next;
            free(entry->key);
            free(entry);
            entry = next;
        }
    }

    free(ht);
}

/* Buffer management */
struct buffer {
    char *data;
    size_t size;
    size_t capacity;
};

struct buffer *buffer_create(size_t initial_capacity) {
    struct buffer *buf = malloc(sizeof(struct buffer));
    if (!buf) return NULL;

    buf->data = malloc(initial_capacity);
    if (!buf->data) {
        free(buf);
        return NULL;
    }

    buf->size = 0;
    buf->capacity = initial_capacity;

    return buf;
}

int buffer_append(struct buffer *buf, const char *data, size_t len) {
    if (buf->size + len > buf->capacity) {
        size_t new_capacity = (buf->capacity * 2) + len;
        char *new_data = realloc(buf->data, new_capacity);
        if (!new_data) return -1;

        buf->data = new_data;
        buf->capacity = new_capacity;
    }

    memcpy(buf->data + buf->size, data, len);
    buf->size += len;

    return 0;
}

void buffer_destroy(struct buffer *buf) {
    if (buf) {
        free(buf->data);
        free(buf);
    }
}
