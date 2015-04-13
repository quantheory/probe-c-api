# `probe-c-api`

[![Build Status](https://travis-ci.org/quantheory/probe-c-api.svg?branch=master)](https://travis-ci.org/quantheory/probe-c-api)

The purpose of `probe-c-api` is to deal with cases where the API of a C library
is known well enough to write C code using the library, but not enough is known
about the interface to write bindings in another language, such as Rust.

In such cases, there are three possible approaches:

 1. Acquire the library and write bindings manually for that specific library.
    This works best if one knows *a priori* that the ABI for a library is very
    stable, whether that is true by design, or simply because the users of a set
    of bindings are likely to all use one particular version in practice.

    This approach generally fails if the bindings must have many variants,
    e.g. to account for different platforms, C compilers, configuration options,
    and/or library versions.

 2. Parse the C header file(s) and extract any information needed. This is the
    approach used by `bindgen`, for instance. This is highly effective for
    getting the correct information for a particular machine that will be
    running software using the target C library.

    One potential problem with this approach is that it tends to be difficult to
    implement. To be both generic and robust, such a system has to include much
    of the functionality of a full C compiler, including the preprocessor,
    parser, and some pieces of the type checker. Perhaps more seriously, such a
    system has to replicate *non-portable* properties of a C compiler,
    e.g. platform-specific integer sizes, or non-standard attributes and other
    syntax extensions.

    Finally, a naive analysis of header files may include macros, types, or
    objects in the generated bindings that are not actually in the documented
    API, leading to unstable items and quasi-private implementation details
    being exposed in the output.

 3. Use the library to compile, then optionally link and run, short test
    functions written in C. By monitoring the exit status and output of each
    step, details of the C library can be probed. This is the approach of
    `probe-c-api`, as well as of many existing build system toolkits, such as
    autotools and CMake.

    The advantage of this approach is that it checks the *actual* behavior of
    programs compiled against the target library with a C compiler, but in a way
    that's guided by knowledge of the *documented* API, which is input by the
    developer of the bindings. This results in a "best of both worlds" situation
    where the ability to create correct bindings is not dependent on hard-coded
    knowledge about the library's implementation, the platform, or the C
    compiler.

    This does come at a bit of a price. Programs that "probe" the API to
    discover implementation details rely heavily on knowledge provided by the
    user. Without directly parsing and interpreting the header, it is not
    usually possible to discover identifiers not specified by the user. It is
    also not possible to get information about types defined in a library,
    e.g. the names, order, and types of fields in a struct. The user needs to
    either already possess this information, or make do with an incomplete
    specification, e.g. knowing the size and minimum alignment of a struct, but
    without knowing anything about its fields.

    Perhaps the biggest disadvantage of this approach is that it does not work
    well for cross-compilation. In order to probe a C library, it is usually
    necessary to be able to run programs linked against it locally.

    Another disadvantage of this approach is that the probe has to rely on many
    "moving parts". Specifically, it has to have a C compiler available, be able
    to invoke that compiler with the correct flags to build a running program,
    be able to run that program in a compatible environment (e.g. dynamically
    linking against the correct libraries), and get the output and return status
    of child processes from the OS.

    While this may not seem like such a high bar to overcome, it is not unheard
    of for such testing to fail on unusual or novel platforms, or with rare C
    compilers. For this reason, `probe-c-api` does *not* invoke a C compiler
    directly, but rather requires the user to provide a method to convert a C
    source file into an object file. Similarly, it does *not* invoke the program
    produced directly, but rather requires the user to provide a method that can
    invoke the program and which returns the output.

    This will hopefully produce a system that's more flexible and modular than
    systems like autotools are (the way most people use them). A library that
    wraps a C compiler can handle compilation, `probe-c-api` can handle
    auto-generating test programs and interpreting the results, standard
    libraries can handle interaction with the OS, and the user can handle any
    quirks relevant to their particular case. In particular, the user can handle
    cross-compilation by running test programs outside of the build platform, by
    sending the executable over the network, to a machine attached as a
    peripheral device, or to an emulator.

    Not coincidentally, by requiring the user to specify how the probe should
    interact with the C compiler and target platform, this design reduces the
    scope of `probe-c-api`, making it much simpler to implement as a standalone
    crate.
