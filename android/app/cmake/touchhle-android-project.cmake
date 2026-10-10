foreach(linker_flags
        CMAKE_EXE_LINKER_FLAGS
        CMAKE_MODULE_LINKER_FLAGS
        CMAKE_SHARED_LINKER_FLAGS)
    set(${linker_flags}
        "${${linker_flags}} -Wl,-z,max-page-size=16384"
        CACHE STRING "" FORCE)
endforeach()
