// jars: izumi-reflect izumi-reflect-boopickle scala-collection-compat
// targets: js interp
//> using dep dev.zio::izumi-reflect:3.0.9
// izumi-reflect's `Tag` of an enum case's singleton (`Aff.Instance.type`, through an alias as a
// cache's key type names it): the macro reads the case's `Symbol.typeRef`, a reference to the
// val whose `underlying` is the enum's type.
import izumi.reflect.Tag

enum Aff:
  case Instance
  case Library(id: Int)

object Aliases:
  type AffInstance = Aff.Instance.type

object Main:
  def main(args: Array[String]): Unit =
    println(Tag[Aliases.AffInstance].tag)
    println(Tag[Aff.Instance.type].tag =:= Tag[Aliases.AffInstance].tag)
    println(Tag[Aff.Library].tag)
    println(Tag[Aff.Instance.type].tag <:< Tag[Aff].tag)
