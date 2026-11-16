package cta

class D extends S(1, 2)
class E extends Prot(9)

package sub:
  object InSub:
    def make(): Int = new cta.S().x + new cta.P3(10).x

@main def main(): Unit =
  println(new P3(3).x)
  println(new P4(4).x)
  println(new S().x)
  println(new S("four").x)
  println(new D().x)
  println(new E().x)
  println(K(5).copy(x = 6))
  println(sub.InSub.make())
  println(InPkg.own())
  println(new Hidden("argument").x)
  println(ctaout.Outside.hidden())
  println(ctaout.Outside.anonymous())
  println(ctaout.Outside.upstream())
