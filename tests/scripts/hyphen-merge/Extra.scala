// A package declared by its source name beside a directory's script package, its encoded name: one package of the
// root, which dotty's PackageScope keys by name.mangled, as Namer.createPackageSymbol reuses it.
package `x-y`
object Answer { val value = 42 }
