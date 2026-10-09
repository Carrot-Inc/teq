// An object whose name is a package object's (`<file>$package`) in a jar, named through an import
// by an inline body of another package: read as the object it is, though the package's members
// are entered only on demand.
package fix.ponforeign {
  object `Token$package`:
    override def toString: String = "ok"
}

package fix.ponapi {
  import fix.ponforeign.`Token$package`

  object PonApi:
    inline def ref: AnyRef = `Token$package`
}
