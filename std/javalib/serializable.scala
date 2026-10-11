// `java.io.Serializable` in a file of its own: the parent `Desugar.classDef` gives every case class
// and enum, which the std's `Option`, `Either`, `List`, `Try`, `Enum` and tuples extend as
// scala-library's do, so that naming it enters this file alone and not the rest of `java.io`, as
// dotty reads the interface from its own class file.
package java.io:

  @jvmClass("java/io/Serializable")
  trait Serializable extends Any
