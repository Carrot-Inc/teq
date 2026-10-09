package cob

import scala.deriving.Mirror
import coa.*

object Use:
  def name(s: Signal): String = s match
    case Stop => "stop"
    case Go => "go"
    case Lights.Amber => "amber"
  def main(args: Array[String]): Unit =
    println(List(Stop, Go, Lights.Amber).map(name).mkString(","))
    val m = summon[Mirror.ProductOf[Stop.type]]
    println(m.fromProduct(EmptyTuple) == Stop)
    println(Go.productPrefix + " " + Go.productArity)
    println(Lights.Amber.toString)
