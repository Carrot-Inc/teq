// `@targetName` of a lone surrogate: the method's name is the literal's code units (`computeTargetName`'s
// `str.toTermName`, SymDenotations.scala:558–568), which the class file is to write encoded over those units as
// `NameTransformer.encode` reads them, `$uD83D` and `a$uD83Db`, as scalac 3.8.4 writes them.
// abi: $uD83D a$uD83Db
package targetnameunits
class TN:
  @scala.annotation.targetName("\uD83D") def f: Int = 1
  @scala.annotation.targetName("a\uD83Db") def g: Int = 2
