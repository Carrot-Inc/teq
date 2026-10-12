// A local enum's companion is a local object, made on its first use in each run of the block, its
// values and lookups with it, as dotty desugars it (`Desugar.classDef`, `DesugarEnums`) and as
// `LazyVals.transformLocalDef` stores a local object: one identity within a run, another in the
// next, the values reading the block's values of their run, a closure keeping its run's enum, an
// explicit companion merged with the generated members.
def identity(): AnyRef =
  enum E:
    case A
  E.A

def captured(n: Int): Int =
  enum E:
    case A
    def get = n
  E.A.get

def classCase(n: Int): () => Int =
  enum E:
    case A
    case B(x: Int)
    def get: Int = this match
      case A => n
      case B(x) => n + x
  () => E.B(1).get + E.A.get

def companion(n: Int): Int =
  enum E:
    case A
  object E:
    def get = n
  E.get

def explicit(): AnyRef =
  enum E:
    case A
  object E
  E.A

def lazily(use: Boolean): AnyRef =
  enum E:
    case A
    println("case")
  println("block")
  if use then E.A else new Object

def lookups(): AnyRef =
  enum E:
    case A, B
  println(E.values(0) eq E.A)
  println(E.valueOf("B") eq E.B)
  println(E.fromOrdinal(1) eq E.B)
  E.A

def mutable(): () => Int =
  var n = 0
  enum E:
    case A
    def get: Int = { n += 1; n }
  () => E.A.get

def objects(): AnyRef =
  object O
  O

def loop(): Int =
  def f(): AnyRef =
    enum E:
      case A
    E.A
  var previous = f()
  var same = 0
  var i = 0
  while i < 1000 do
    val next = f()
    if previous eq next then same += 1
    previous = next
    i += 1
  same

def parameterised(k: Int): String =
  enum Planet(val mass: Int):
    case Small extends Planet(k)
    case Large extends Planet(k * 10)
    def heavier = mass > k
  Planet.values.map(p => s"${p.toString}:${p.mass}:${p.heavier}:${p.ordinal}").mkString(",")

@main def run(): Unit =
  println(identity() eq identity())
  println(captured(7))
  println(captured(8))
  val a = classCase(2)
  val b = classCase(10)
  println(a())
  println(b())
  println(companion(7))
  println(companion(8))
  println(explicit() eq explicit())
  lazily(false)
  lazily(true)
  lazily(true)
  println(lookups() eq lookups())
  val m1 = mutable()
  val m2 = mutable()
  println(m1())
  println(m1())
  println(m2())
  println(objects() eq objects())
  println(loop())
  println(parameterised(3))
