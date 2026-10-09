// `import v.*` from a stable val of an object and of a package: the conversion the val's class
// defines is called on that val, selected through its object or package (scalac: "object:7",
// "package:8", "object").
object UseObject:
  import dsls.Config.dsl.*
  def run(): String =
    val s: String = 7
    s
object UsePackage:
  import dsls.pkgDsl.*
  def run(): String =
    val s: String = 8
    s
@main def main(): Unit =
  println(UseObject.run())
  println(UsePackage.run())
  import dsls.Config.dsl.*
  println(name)
