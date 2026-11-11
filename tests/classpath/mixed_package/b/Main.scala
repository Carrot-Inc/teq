import scala2lib.codecs.*
import scala2lib.ops.{*, given}
@main def run(): Unit =
  println(summon[Boolean])
  println(List(1, 2).asLine(','))
