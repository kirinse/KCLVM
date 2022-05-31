use std::process::Command;

fn main() {
    {
        // compile our linker
        let cxxflags = Command::new("llvm-config")
            .args(&["--cxxflags"])
            .output()
            .expect("could not execute llvm-config");

        let cxxflags = String::from_utf8(cxxflags.stdout).unwrap();

        let mut build = cc::Build::new();

        build.file("src/linker/linker.cpp").cpp(true);

        if !cfg!(target_os = "windows") {
            build.flag("-Wno-unused-parameter");
        }

        for flag in cxxflags.split_whitespace() {
            build.flag(flag);
        }

        build.compile("kclvm_clang");

        // add the llvm linker
        let libdir = Command::new("llvm-config")
            .args(&["--libdir"])
            .output()
            .unwrap();
        let libdir = String::from_utf8(libdir.stdout).unwrap();

        println!("cargo:libdir={}", libdir);
        for lib in &[
            "lldELF",
            "lldDriver",
            "lldCore",
            "lldCommon",
            "lldWasm",
            "clangAST",
            "clangBasic",
            "clangDriver",
            "clangEdit",
            "clangFrontend",
            "clangFrontendTool",
            "clangTooling",
            "LLVMDlltoolDriver",
            "clangToolingASTDiff",
            "clangToolingCore",
            "clangToolingInclusions",
            "clangToolingRefactoring",
            "clangToolingSyntax",
            "clangSerialization",
            "clangLex",
            /*
            "clangAPINotes",
            "clangARCMigrate",
            "clangASTMatchers",
            "clangAnalysis",
            "clangApplyReplacements",
            "clangChangeNamespace",
            "clangCodeGen",
            "clangCrossTU",
            "clangDaemon",
            "clangDaemonTweaks",
            "clangDependencyScanning",
            "clangDirectoryWatcher",
            "clangDoc",
            "clangDynamicASTMatchers",
            "clangFormat",
            "clangHandleCXX",
            "clangHandleLLVM",
            "clangIncludeFixer",
            "clangIncludeFixerPlugin",
            "clangIndex",
            "clangIndexSerialization",
            "clangMove",
            "clangParse",
            "clangQuery",
            "clangReorderFields",
            "clangRewrite",
            "clangRewriteFrontend",
            "clangSema",
            "clangStaticAnalyzerCheckers",
            "clangStaticAnalyzerCore",
            "clangStaticAnalyzerFrontend",
            "clangTesting",
            "clangTidy",
            "clangTidyAbseilModule",
            "clangTidyAlteraModule",
            "clangTidyAndroidModule",
            "clangTidyBoostModule",
            "clangTidyBugproneModule",
            "clangTidyCERTModule",
            "clangTidyConcurrencyModule",
            "clangTidyCppCoreGuidelinesModule",
            "clangTidyDarwinModule",
            "clangTidyFuchsiaModule",
            "clangTidyGoogleModule",
            "clangTidyHICPPModule",
            "clangTidyLLVMLibcModule",
            "clangTidyLLVMModule",
            "clangTidyLinuxKernelModule",
            "clangTidyMPIModule",
            "clangTidyMain",
            "clangTidyMiscModule",
            "clangTidyModernizeModule",
            "clangTidyObjCModule",
            "clangTidyOpenMPModule",
            "clangTidyPerformanceModule",
            "clangTidyPlugin",
            "clangTidyPortabilityModule",
            "clangTidyReadabilityModule",
            "clangTidyUtils",
            "clangTidyZirconModule",
            "clangTransformer",
            "clangdRemoteIndex",
            "clangdSupport",
            "clangdXpcJsonConversions",
            "clangdXpcTransport",
            */
        ] {
            println!("cargo:rustc-link-lib=static={}", lib);
        }

        // And all the symbols were not using, needed by Windows and debug builds
        for lib in &["lldReaderWriter", "lldMachO", "lldYAML"] {
            println!("cargo:rustc-link-lib=static={}", lib);
        }
    }

    let output = Command::new("git")
        .args(&["describe", "--tags"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    // Make sure we have an 8MiB stack on Windows. Windows defaults to a 1MB
    // stack, which is not big enough for debug builds
    #[cfg(windows)]
    println!("cargo:rustc-link-arg=/STACK:8388608");

    println!("cargo:rerun-if-changed=src/linker/linker.cpp");
}
