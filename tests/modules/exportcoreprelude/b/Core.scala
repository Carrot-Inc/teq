package xcore.lilaism

// lila.core's shape: a trait exports the library's newtypes, an object inherits them, and the
// package's top-level clause exports the object's members, types among them; a class of the
// package names one in its header. The trait's forwarders are made before the package's, which
// a lookup of the package's entries makes (a qualified package clause sees `xcore.lilaism`'s
// members, not `xcore`'s).
trait CoreExports:
  export xsl.newtypes.{given, *}

object Core extends CoreExports
