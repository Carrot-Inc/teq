// jars: izumi-reflect izumi-reflect-boopickle scala-collection-compat
// targets: js interp
//> using dep dev.zio::izumi-reflect:3.0.9
// izumi-reflect's tag of a top-level opaque type and of a top-level alias, named through their
// file's `$package` object as scalac's `TypeRef` prefixes them (`Desugar.packageDef`): the
// inspector's qualifier is that object's `TermRef`, `izop.izumi_opaque_prefix$package::Aid`.
package izop

import izumi.reflect.Tag

opaque type Aid = String
object Aid:
  def apply(s: String): Aid = s

type Pair = (Int, Int)

object Main:
  def main(args: Array[String]): Unit =
    println(Tag[Aid].tag.toString)
    println(Tag[Aid].tag.repr)
    println(Tag[Pair].tag.toString)
    println(Tag[List[Aid]].tag.repr)
