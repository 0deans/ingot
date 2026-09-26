/*
 * LD_PRELOAD shim for the JVM inside the Alpine (musl) PRoot sandbox on Android.
 *
 * The JDK launcher starts the VM on a new thread created with guardsize 0. Without a
 * guard page, the kernel merges later anonymous mappings into that thread's stack VMA.
 * HotSpot's startup PaX probe mprotect()s such a page to RWX, which SELinux then
 * checks as `execstack` - denied for all Android apps - and the VM aborts with
 * "Failed to mark memory page as executable - check if grsecurity/PaX is enabled".
 * Forcing a guard region keeps thread stacks in their own VMA.
 *
 * Built libc-free (the symbol is resolved via musl's dlsym at runtime):
 *   clang --target=aarch64-linux-gnu -O2 -fPIC -shared -nostdlib -ffreestanding \
 *         -fno-stack-protector -o stackguard-aarch64.so stackguard.c
 */
typedef unsigned long size_t;
void *dlsym(void *handle, const char *name);
#define RTLD_NEXT ((void *)-1)

int pthread_attr_setguardsize(void *attr, size_t size) {
  static int (*real)(void *, size_t);
  if (!real) real = (int (*)(void *, size_t))dlsym(RTLD_NEXT, "pthread_attr_setguardsize");
  if (size < 65536) size = 65536;
  return real(attr, size);
}
