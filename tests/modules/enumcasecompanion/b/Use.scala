package ecb

import scala.deriving.Mirror
import eca.*

@main def run(): Unit =
  val m = summon[Mirror.ProductOf[Draft.Email]]
  println((m eq summon[Mirror.ProductOf[Draft.Email]], m eq Here.email, m.fromProduct((Some("a"), true))))
  println(Draft.Email)
  val e = Draft.Email(Some("s"))
  val ds: List[Draft] = List(Draft.Store(None), e)
  println(ds)
  println(e match { case Draft.Email(Some(s), on) => s"$s $on"; case _ => "other" })
  val o = new Outer
  val a = summon[Mirror.ProductOf[o.In.A]]
  println((a eq summon[Mirror.ProductOf[o.In.A]], a.fromProduct(Tuple1(4)), o.In.A(5)))
