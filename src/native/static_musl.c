// The companion of a fully static x86_64-linux-musl executable over Scala Native 0.5.12's C runtime: an object of
// the link, with -Wl,-wrap=pthread_create -Wl,-wrap=main (src/native/build.rs).
//
// 1. Thread stack bounds.  nativeThreadTLS.c (detectStackBounds) reaches pthread_getattr_np only through
//    dlsym(RTLD_DEFAULT, ..) or dlopen("libpthread.so.0") + dlsym.  In a static musl executable both are stubs that
//    fail; the fallback reads /proc/self/maps, where a thread's stack is merged with the mapping above it, and the
//    overflow guard page lands in that neighbour.  The three functions below answer that one lookup.
//
// 2. Stack walks.  musl's program and thread entry code (_start, libc_start_main_stage2, start, __clone) carries no
//    unwind information, so every walk that reaches the outermost frame (each Throwable's stack trace) makes
//    libunwind scan the whole .eh_frame linearly.  The two entry frames below are marked as outermost
//    (.cfi_undefined rip), which is what glibc's own entry code does, and the walk ends there.
//
// 3. Names in stack traces.  The runtime names a frame with dladdr (unwind.c), which a static musl executable answers
//    with "nothing found".  The dladdr below reads the function symbols of /proc/self/exe once; it needs /proc and an
//    executable that was not stripped, and answers "nothing found" otherwise.
//
// 4. Trust roots, in a link with libcrypto and -Wl,-wrap=X509_STORE_set_default_paths -Wl,-wrap=X509_verify_cert.
//    s2n-tls trusts libcrypto's default places alone ($SSL_CERT_FILE or OPENSSLDIR/cert.pem, $SSL_CERT_DIR or the
//    hashed OPENSSLDIR/certs), which miss the bundle of several distributions.  When neither variable is set and the
//    default places hold no certificate, the first of the distributions' places that does is added to the store.  A
//    verification that fails for want of an issuer while no place held a certificate says so once on stderr.  The
//    references to libcrypto are weak, so a program linked without it still links and carries none of it, and a
//    libcrypto that lacks one of the functions keeps its own behaviour.
#define _GNU_SOURCE
#include <dirent.h>
#include <dlfcn.h>
#include <elf.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

static char pthread_handle;

void *dlopen(const char *file, int mode) {
    (void)mode;
    return file != NULL && strcmp(file, "libpthread.so.0") == 0 ? &pthread_handle : NULL;
}

void *dlsym(void *restrict handle, const char *restrict name) {
    (void)handle;
    return strcmp(name, "pthread_getattr_np") == 0 ? (void *)pthread_getattr_np : NULL;
}

int dlclose(void *handle) {
    (void)handle;
    return 0;
}

struct function {
    uintptr_t start, end;
    const char *name;
};

static struct function *functions;
static size_t function_count;
static pthread_once_t functions_once = PTHREAD_ONCE_INIT;

static int by_start(const void *a, const void *b) {
    uintptr_t x = ((const struct function *)a)->start, y = ((const struct function *)b)->start;
    return x < y ? -1 : x > y;
}

static void load_functions(void) {
    int fd = open("/proc/self/exe", O_RDONLY | O_CLOEXEC);
    if (fd < 0)
        return;
    struct stat st;
    const unsigned char *image = MAP_FAILED;
    if (fstat(fd, &st) == 0 && (size_t)st.st_size > sizeof(Elf64_Ehdr))
        image = mmap(NULL, (size_t)st.st_size, PROT_READ, MAP_PRIVATE, fd, 0);
    close(fd);
    if (image == MAP_FAILED)
        return;
    const Elf64_Ehdr *header = (const Elf64_Ehdr *)image;
    // ET_EXEC only: the symbols' values are the run-time addresses.
    if (memcmp(header->e_ident, ELFMAG, SELFMAG) != 0 || header->e_type != ET_EXEC || header->e_shoff == 0)
        return;
    const Elf64_Shdr *sections = (const Elf64_Shdr *)(image + header->e_shoff);
    for (unsigned i = 0; i < header->e_shnum; i++) {
        if (sections[i].sh_type != SHT_SYMTAB)
            continue;
        const Elf64_Sym *symbols = (const Elf64_Sym *)(image + sections[i].sh_offset);
        size_t symbol_count = sections[i].sh_size / sizeof(Elf64_Sym);
        const char *names = (const char *)(image + sections[sections[i].sh_link].sh_offset);
        struct function *table = malloc(symbol_count * sizeof *table);
        if (table == NULL)
            return;
        size_t count = 0;
        for (size_t j = 0; j < symbol_count; j++) {
            const Elf64_Sym *symbol = &symbols[j];
            if (ELF64_ST_TYPE(symbol->st_info) == STT_FUNC && symbol->st_size != 0 && symbol->st_shndx != SHN_UNDEF)
                table[count++] = (struct function){symbol->st_value, symbol->st_value + symbol->st_size,
                                                   names + symbol->st_name};
        }
        qsort(table, count, sizeof *table, by_start);
        functions = table;
        function_count = count;
        return; // the mapping stays: the names point into it
    }
}

int dladdr(const void *address, Dl_info *info) {
    pthread_once(&functions_once, load_functions);
    uintptr_t wanted = (uintptr_t)address;
    size_t low = 0, high = function_count;
    while (low < high) {
        size_t middle = low + (high - low) / 2;
        if (functions[middle].start <= wanted)
            low = middle + 1;
        else
            high = middle;
    }
    if (low == 0 || wanted >= functions[low - 1].end)
        return 0;
    info->dli_fname = "/proc/self/exe";
    info->dli_fbase = NULL;
    info->dli_sname = functions[low - 1].name;
    info->dli_saddr = (void *)functions[low - 1].start;
    return 1;
}

typedef struct x509_store_st X509_STORE;
typedef struct x509_store_ctx_st X509_STORE_CTX;
typedef struct stack_st OPENSSL_STACK;

int __real_X509_STORE_set_default_paths(X509_STORE *store) __attribute__((weak));
int __real_X509_verify_cert(X509_STORE_CTX *context) __attribute__((weak));
const char *X509_get_default_cert_file(void) __attribute__((weak));
const char *X509_get_default_cert_dir(void) __attribute__((weak));
const char *X509_get_default_cert_file_env(void) __attribute__((weak));
const char *X509_get_default_cert_dir_env(void) __attribute__((weak));
int X509_STORE_load_locations(X509_STORE *store, const char *file, const char *directory) __attribute__((weak));
int X509_STORE_lock(X509_STORE *store) __attribute__((weak));
int X509_STORE_unlock(X509_STORE *store) __attribute__((weak));
OPENSSL_STACK *X509_STORE_get0_objects(const X509_STORE *store) __attribute__((weak));
int OPENSSL_sk_num(const OPENSSL_STACK *stack) __attribute__((weak));
X509_STORE *X509_STORE_CTX_get0_store(const X509_STORE_CTX *context) __attribute__((weak));
int X509_STORE_CTX_get_error(const X509_STORE_CTX *context) __attribute__((weak));
void ERR_clear_error(void) __attribute__((weak));

// The distributions' places after Go's crypto/x509/root_linux.go (certFiles, certDirectories), with curl's
// configure-time search (acinclude.m4, CURL_CHECK_CA_BUNDLE, and its capath) merged in.
static const char *const distribution_bundles[] = {
    "/etc/ssl/certs/ca-certificates.crt",                // Debian, Ubuntu, Gentoo (Go, curl)
    "/etc/pki/tls/certs/ca-bundle.crt",                  // Fedora, RHEL 6 (Go, curl)
    "/etc/ssl/ca-bundle.pem",                            // openSUSE (Go)
    "/etc/pki/tls/cacert.pem",                           // OpenELEC (Go)
    "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem", // CentOS, RHEL 7 (Go)
    "/usr/share/ssl/certs/ca-bundle.crt",                // old Red Hat (curl)
    "/etc/ssl/cert.pem",                                 // Alpine (Go, curl)
};
static const char *const distribution_directories[] = {
    "/etc/ssl/certs",     // SLES 10 and 11 (Go), curl's capath
    "/etc/pki/tls/certs", // Fedora, RHEL (Go)
};
#define COUNT(array) (sizeof(array) / sizeof *(array))

// OpenSSL 3 defines each of them in the archive member of a wrapped function or in one that member needs.
static bool libcrypto_has_all(void) {
    return X509_get_default_cert_file != NULL && X509_get_default_cert_dir != NULL &&
           X509_get_default_cert_file_env != NULL && X509_get_default_cert_dir_env != NULL &&
           X509_STORE_load_locations != NULL && X509_STORE_lock != NULL && X509_STORE_unlock != NULL &&
           X509_STORE_get0_objects != NULL && OPENSSL_sk_num != NULL && X509_STORE_CTX_get0_store != NULL &&
           X509_STORE_CTX_get_error != NULL && ERR_clear_error != NULL;
}

static atomic_bool no_place_held_certificates;
static atomic_flag no_certificates_reported = ATOMIC_FLAG_INIT;

struct places {
    const char *file_variable, *directory_variable;
    const char *file, *directory;
};

// The places X509_STORE_set_default_paths reads, decided as libcrypto decides them (ossl_safe_getenv).
static struct places default_places(void) {
    const char *file = secure_getenv(X509_get_default_cert_file_env());
    const char *directory = secure_getenv(X509_get_default_cert_dir_env());
    return (struct places){file, directory, file != NULL ? file : X509_get_default_cert_file(),
                           directory != NULL ? directory : X509_get_default_cert_dir()};
}

// The names libcrypto's directory lookup opens (c_rehash's): eight lowercase hex digits, '.', a decimal suffix.
static bool is_hash_name(const char *name) {
    for (int i = 0; i < 8; i++)
        if (!((name[i] >= '0' && name[i] <= '9') || (name[i] >= 'a' && name[i] <= 'f')))
            return false;
    if (name[8] != '.' || name[9] < '0' || name[9] > '9')
        return false;
    for (const char *p = name + 10; *p != '\0'; p++)
        if (*p < '0' || *p > '9')
            return false;
    return true;
}

static bool directory_holds_certificates(const char *path, size_t length) {
    char name[PATH_MAX];
    if (length == 0 || length >= sizeof name)
        return false;
    memcpy(name, path, length);
    name[length] = '\0';
    DIR *directory = opendir(name);
    if (directory == NULL)
        return false;
    bool found = false;
    for (struct dirent *entry; !found && (entry = readdir(directory)) != NULL;)
        found = is_hash_name(entry->d_name);
    closedir(directory);
    return found;
}

// A list split as libcrypto's directory lookup splits it.
static bool directories_hold_certificates(const char *list) {
    for (const char *start = list;; ) {
        const char *end = strchrnul(start, ':');
        if (directory_holds_certificates(start, (size_t)(end - start)))
            return true;
        if (*end == '\0')
            return false;
        start = end + 1;
    }
}

static int store_size(X509_STORE *store) {
    if (store == NULL || X509_STORE_lock(store) != 1)
        return -1;
    int size = OPENSSL_sk_num(X509_STORE_get0_objects(store));
    X509_STORE_unlock(store);
    return size;
}

// Bundles first, then hashed directories; the defaults were read already.
static bool add_distribution_place(X509_STORE *store, const struct places *defaults) {
    for (size_t i = 0; i < COUNT(distribution_bundles); i++)
        if (strcmp(distribution_bundles[i], defaults->file) != 0 &&
            X509_STORE_load_locations(store, distribution_bundles[i], NULL) == 1)
            return true;
    for (size_t i = 0; i < COUNT(distribution_directories); i++) {
        const char *directory = distribution_directories[i];
        if (strcmp(directory, defaults->directory) != 0 && directory_holds_certificates(directory, strlen(directory)) &&
            X509_STORE_load_locations(store, NULL, directory) == 1)
            return true;
    }
    return false;
}

int __wrap_X509_STORE_set_default_paths(X509_STORE *store) {
    if (!libcrypto_has_all())
        return __real_X509_STORE_set_default_paths(store);
    int saved_errno = errno;
    int before = store_size(store);
    int result = __real_X509_STORE_set_default_paths(store);
    if (result == 1) {
        struct places defaults = default_places();
        bool held = store_size(store) > before || directories_hold_certificates(defaults.directory);
        if (!held && defaults.file_variable == NULL && defaults.directory_variable == NULL) {
            held = add_distribution_place(store, &defaults);
            ERR_clear_error();
        }
        atomic_store(&no_place_held_certificates, !held);
    }
    errno = saved_errno;
    return result;
}

struct line {
    char text[2048];
    size_t length;
};

static void append(struct line *line, const char *text) {
    size_t room = sizeof line->text - 1 - line->length, length = strlen(text);
    if (length > room)
        length = room;
    memcpy(line->text + line->length, text, length);
    line->length += length;
}

static void append_place(struct line *line, const char *variable, const char *value) {
    if (line->length > 0)
        append(line, ", ");
    if (variable != NULL) {
        append(line, variable);
        append(line, "=");
    }
    append(line, value);
}

static void report_no_certificates(void) {
    struct places defaults = default_places();
    struct line places = {.length = 0};
    append_place(&places, defaults.file_variable != NULL ? X509_get_default_cert_file_env() : NULL, defaults.file);
    append_place(&places, defaults.directory_variable != NULL ? X509_get_default_cert_dir_env() : NULL, defaults.directory);
    if (defaults.file_variable == NULL && defaults.directory_variable == NULL) {
        for (size_t i = 0; i < COUNT(distribution_bundles); i++)
            if (strcmp(distribution_bundles[i], defaults.file) != 0)
                append_place(&places, NULL, distribution_bundles[i]);
        for (size_t i = 0; i < COUNT(distribution_directories); i++)
            if (strcmp(distribution_directories[i], defaults.directory) != 0)
                append_place(&places, NULL, distribution_directories[i]);
    }
    places.text[places.length] = '\0';
    struct line line = {.length = 0};
    append(&line, program_invocation_short_name);
    append(&line, ": no CA certificates found in ");
    append(&line, places.text);
    append(&line, "; install the system's CA certificates or set ");
    append(&line, X509_get_default_cert_file_env());
    append(&line, " or ");
    append(&line, X509_get_default_cert_dir_env());
    line.text[line.length++] = '\n';
    for (size_t written = 0; written < line.length;) {
        ssize_t n = write(STDERR_FILENO, line.text + written, line.length - written);
        if (n < 0 && errno == EINTR)
            continue;
        if (n <= 0)
            return;
        written += (size_t)n;
    }
}

static bool for_want_of_an_issuer(int error) {
    switch (error) {
    case 2:  // X509_V_ERR_UNABLE_TO_GET_ISSUER_CERT
    case 18: // X509_V_ERR_DEPTH_ZERO_SELF_SIGNED_CERT
    case 19: // X509_V_ERR_SELF_SIGNED_CERT_IN_CHAIN
    case 20: // X509_V_ERR_UNABLE_TO_GET_ISSUER_CERT_LOCALLY
        return true;
    default:
        return false;
    }
}

int __wrap_X509_verify_cert(X509_STORE_CTX *context) {
    int result = __real_X509_verify_cert(context);
    if (result <= 0 && context != NULL && atomic_load(&no_place_held_certificates) &&
        for_want_of_an_issuer(X509_STORE_CTX_get_error(context)) && store_size(X509_STORE_CTX_get0_store(context)) == 0 &&
        !atomic_flag_test_and_set(&no_certificates_reported)) {
        int saved_errno = errno;
        report_no_certificates();
        errno = saved_errno;
    }
    return result;
}

struct thread_start {
    void *(*routine)(void *);
    void *arg;
};

void *sn_static_thread_body(void *packed) {
    struct thread_start start = *(struct thread_start *)packed;
    free(packed);
    return start.routine(start.arg);
}

void *sn_static_thread_entry(void *packed);
int __real_pthread_create(pthread_t *restrict thread, const pthread_attr_t *restrict attr, void *(*routine)(void *),
                          void *restrict arg);

int __wrap_pthread_create(pthread_t *restrict thread, const pthread_attr_t *restrict attr, void *(*routine)(void *),
                          void *restrict arg) {
    struct thread_start *start = malloc(sizeof *start);
    if (start == NULL)
        return EAGAIN;
    start->routine = routine;
    start->arg = arg;
    int result = __real_pthread_create(thread, attr, sn_static_thread_entry, start);
    if (result != 0)
        free(start);
    return result;
}

__asm__(".text\n"
        ".globl sn_static_thread_entry\n"
        ".type sn_static_thread_entry,@function\n"
        "sn_static_thread_entry:\n"
        "    .cfi_startproc\n"
        "    .cfi_undefined rip\n"
        "    subq $8, %rsp\n"
        "    .cfi_adjust_cfa_offset 8\n"
        "    call sn_static_thread_body\n"
        "    addq $8, %rsp\n"
        "    .cfi_adjust_cfa_offset -8\n"
        "    ret\n"
        "    .cfi_endproc\n"
        ".size sn_static_thread_entry, .-sn_static_thread_entry\n"
        ".globl __wrap_main\n"
        ".type __wrap_main,@function\n"
        "__wrap_main:\n"
        "    .cfi_startproc\n"
        "    .cfi_undefined rip\n"
        "    subq $8, %rsp\n"
        "    .cfi_adjust_cfa_offset 8\n"
        "    call __real_main\n"
        "    addq $8, %rsp\n"
        "    .cfi_adjust_cfa_offset -8\n"
        "    ret\n"
        "    .cfi_endproc\n"
        ".size __wrap_main, .-__wrap_main\n");
