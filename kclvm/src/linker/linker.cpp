// Call the Clang linker

// https://clang.llvm.org/docs/LibTooling.html

#include "lld/Common/Driver.h"

#include "clang/Frontend/FrontendActions.h"
#include "clang/Tooling/Tooling.h"
#include "clang/Tooling/CommonOptionsParser.h"
#include "llvm/Support/CommandLine.h"


using namespace clang::tooling;

// Apply a custom category to all command-line options so that they are the
// only ones displayed.
static llvm::cl::OptionCategory MyToolCategory("kclvm-clang-tool options");

extern "C" bool LLDWasmLink(const char *argv[], size_t length) {
	llvm::ArrayRef<const char *> args(argv, length);
	return lld::wasm::link(args, false, llvm::outs(), llvm::errs());
}


int main222(int argc, const char **argv) {

    if(argc == 100) {
        //LLDWasmLink(argv, argc);
    }

    
    //CommonOptionsParser OptionsParser(argc, argv, MyToolCategory);
    
    std::vector<std::string> Sources;
    Sources.push_back("hello.c");

    /*
    // We hand the CompilationDatabase we created and the sources to run over into
    // the tool constructor.
    ClangTool Tool(OptionsParser.getCompilations(), Sources);

    // The ClangTool needs a new FrontendAction for each translation unit we run
    // on.  Thus, it takes a FrontendActionFactory as parameter.  To create a
    // FrontendActionFactory from a given FrontendAction type, we call
    // newFrontendActionFactory<clang::SyntaxOnlyAction>().
    int result = Tool.run(newFrontendActionFactory<clang::SyntaxOnlyAction>().get());

    */
    return 0;
}


extern "C" bool ClangLink(const char *argv[], size_t length) {
	llvm::ArrayRef<const char *> args(argv, length);
	return lld::wasm::link(args, false, llvm::outs(), llvm::errs());
}


extern "C" int ClangMain(int argc, const char **argv) {
    printf("hello ClangMain\n");
    return main222(argc, argv);
}