package tfkb

import tfka.*

class K extends Kinds
class D extends Over:
  override val o: Int = 20
class S extends Says

@main def run(): Unit =
  val k = new K
  println(k.bumpP() + k.bumpP())
  println(k.privates)
  println(k.fin + k.v)
  k.Counter.n += 1
  k.Counter.n += 1
  println((k.Counter.n, k.Counter eq k.Counter))
  println((k.label eq k.label, summon[Label](using k.label).text))
  println(new D().o)
  new S
