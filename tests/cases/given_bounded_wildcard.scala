// A given found for a target with a bounded wildcard argument, as chimney's macro searches
// for a `TransformerConfiguration[? <: TransformerFlags]`.
sealed trait Flags
object Flags:
  final class Enable[F, Tail <: Flags] extends Flags
  final class Default extends Flags
final class Config[F <: Flags](val name: String)
object Config:
  implicit val default: Config[Flags.Default] = new Config("default")
object Main:
  def main(args: Array[String]): Unit =
    println(summon[Config[? <: Flags]].name)
    println(implicitly[Config[?]].name)
