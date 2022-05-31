// Call the LLD linker
#include "lld/Common/Driver.h"

extern "C" bool LLDWasmLink(const char *argv[], size_t length)
{
	llvm::ArrayRef<const char *> args(argv, length);

	return lld::wasm::link(args, false, llvm::outs(), llvm::errs());
}

extern "C" bool LLDDarwinLink(const char *argv[], size_t length)
{
	llvm::ArrayRef<const char *> args(argv, length);

	return lld::mach_o::link(args, false, llvm::outs(), llvm::errs());
}

extern "C" bool LLDDarwinNewLink(const char *argv[], size_t length)
{
	llvm::ArrayRef<const char *> args(argv, length);

	return lld::macho::link(args, false, llvm::outs(), llvm::errs());
}
extern "C" bool LLDGnuLink(const char *argv[], size_t length)
{
	llvm::ArrayRef<const char *> args(argv, length);

	return lld::elf::link(args, false, llvm::outs(), llvm::errs());
}
extern "C" bool LLDMingwLink(const char *argv[], size_t length)
{
	llvm::ArrayRef<const char *> args(argv, length);

	return lld::mingw::link(args, false, llvm::outs(), llvm::errs());
}
