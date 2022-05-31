// Call the Clang linker

// https://clang.llvm.org/docs/LibTooling.html

#include "lld/Common/Driver.h"

/*
extern "C"
int lldMain_gnu(int argc, const char **argv) {
	std::vector<const char *> args(argv, argv + argc);
	return !lld::elf::link(args, false, llvm::outs(), llvm::errs());
}

extern "C"
int lldMain_gnu_pe(int argc, const char **argv) {
	std::vector<const char *> args(argv, argv + argc);
	return !lld::mingw::link(args, false, llvm::outs(), llvm::errs());
}

extern "C"
int lldMain_darwin(int argc, const char **argv) {
	std::vector<const char *> args(argv, argv + argc);
	return !lld::mach_o::link(args, false, llvm::outs(), llvm::errs());
}

extern "C"
int lldMain_darwin_new(int argc, const char **argv) {
	std::vector<const char *> args(argv, argv + argc);
	return !lld::macho::link(args, false, llvm::outs(), llvm::errs());
}
*/

extern "C"
int lldMain_wasm(int argc, const char **argv) {
	std::vector<const char *> args(argv, argv + argc);
	return !lld::wasm::link(args, false, llvm::outs(), llvm::errs());
}

extern "C" bool LLDWasmLink(const char *argv[], size_t length)
{
	llvm::ArrayRef<const char *> args(argv, length);

	return lld::wasm::link(args, false, llvm::outs(), llvm::errs());
}
