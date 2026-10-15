package cnu

import scala.deriving.Mirror
import scala.compiletime.constValue

// A mirror an inline method takes for its types alone, its binding dropped where the argument is
// elideable (`Inliner.dropUnusedDefs`): a companion's reference is pure where its class runs no
// initialiser (`TreeInfo.refPurity`), the same over the products as in one build; the one that runs
// a statement stays.
object Use:
  inline def arity[A](using m: Mirror.ProductOf[A]): Int = constValue[Tuple.Size[m.MirroredElemTypes]]
  def id: Int = arity[cni.Models.Id]
  def pair: Int = arity[cni.Models.Pair]
  def loud: Int = arity[cni.Models.Loud]
  def tagged: Int = arity[cni.Models.Tagged]

@main def run(): Unit = println(s"${Use.id} ${Use.pair} ${Use.loud} ${Use.tagged}")
