// A top-level opaque type's owner is its file's `$package` object (`Desugar.packageDef`), the
// module class its qualifier's `termSymbol` names: one symbol to reflection (`TreePickler` 207–246).
@main def run(): Unit = println(Inspect.ownerOf[own.Id])
