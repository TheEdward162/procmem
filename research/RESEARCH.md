## Platform info

### ptrace (linux/macos/unix)

Ptracing procesess on unix systems:

_Note: macOS API slightly differs_

https://linux.die.net/man/2/waitpid

https://linux.die.net/man/2/ptrace

### linux vm_readv

Alternative to procfs on linux:

https://man7.org/linux/man-pages/man2/process_vm_readv.2.html

### macOS

macOS Virtual Memory APIs:

https://stackoverflow.com/questions/1627998/retrieving-the-memory-map-of-its-own-process-in-os-x-10-5-10-6

https://web.mit.edu/darwin/src/modules/xnu/osfmk/man/vm_region.html

https://developer.apple.com/documentation/kernel/mach/mach_vm

ptrace research:

https://alexomara.com/blog/defeating-anti-debug-techniques-macos-ptrace-variants/

https://www.spaceflint.com/?p=150

https://web.mit.edu/darwin/src/modules/xnu/osfmk/man/mach_msg.html

### mach kernel

https://github.com/opensource-apple/xnu

May or may not apply to macOS mach kernel:

https://www.gnu.org/software/hurd/gnumach-doc/Virtual-Memory-Interface.html#Virtual-Memory-Interface

### virtual memory mappings in gnu project

How to enumerate virtual memory mappings on many platforms:

https://git.savannah.gnu.org/gitweb/?p=gnulib.git;a=blob;f=lib/vma-iter.c;h=809828b765332c46a8d6a0d91b83d621f8c50648;hb=HEAD

## Prior work

Only handles raw memory writing, has different api design:
https://github.com/Tommoa/rs-process-memory
