// jars: scala-library izumi-reflect-jvm izumi-reflect-boopickle-jvm
// std: scala-library
//> using dep dev.zio::izumi-reflect:3.0.9
// izumi's `Tag` of a structural type: its macro reads the refinement's members through
// reflection (`Refinement.name`, `.info`, `ByNameType`, `MethodType`) and rebuilds them with
// `Refinement(parent, name, info)` for the tag's own type.
import izumi.reflect.Tag
trait S:
  def n: Int
trait Branches:
  def get(id: Long): Option[String]
  def name(id: Long): String
object Main:
  def main(args: Array[String]): Unit =
    println(summon[Tag[S { def n: Int }]].tag)
    println(summon[Tag[Branches { def get(id: Long): Option[String]; def name(id: Long): String }]].tag)
    println(summon[Tag[S { val v: String; type T = Int }]].tag)
