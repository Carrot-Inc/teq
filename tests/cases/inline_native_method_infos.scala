trait NativeInfoBox:
  type Value
  val value: Value

inline def nativeInfoBound[A](value: A): A =
  def local[B <: A](arg: B): B = arg
  local(value)

inline def nativeInfoLocalBound[A](value: A): A =
  def local[B, C <: B](arg: C): B = arg
  local[A, A](value)

inline def nativeInfoDependent(box: NativeInfoBox): box.Value =
  def local(first: NativeInfoBox)(second: first.Value): first.Value = second
  local(box)(box.value)

inline def nativeInfoClass[A](value: A): A =
  class Local(val stored: A)
  new Local(value).stored

inline def nativeInfoNested[A](value: A): A =
  class Outer:
    class Inner(val stored: A)
    def result: A = new Inner(value).stored
  new Outer().result

inline def nativeInfoCarriers(value: Int): Int =
  def local(first: => Int)(rest: Int*)(using extra: Int): Int =
    first + rest.sum + extra
  local(value)(2, 3)(using 4)

inline def nativeInfoPlain(value: Int): Int =
  def local(arg: Int): Int = arg + 1
  local(value)

trait NativeInfoFamily[A]:
  type Chosen[X] = A match
    case Int => X
    case _ => String
  inline def direct(value: Any): Any =
    def local(arg: Chosen[Int]): Chosen[Int] = arg
    local(value.asInstanceOf[Chosen[Int]])
  inline def poly(value: Any): Any =
    def local[X](arg: Chosen[X]): Chosen[X] = arg
    local[Int](value.asInstanceOf[Chosen[Int]])

object NativeInfoNumbers extends NativeInfoFamily[Int]
object NativeInfoWords extends NativeInfoFamily[String]

@main def inlineNativeMethodInfos(): Unit =
  val box = new NativeInfoBox:
    type Value = Int
    val value = 12
  println(nativeInfoPlain(3))
  println(nativeInfoPlain(8))
  println(nativeInfoBound(7))
  println(nativeInfoBound("bound"))
  println(nativeInfoLocalBound(11))
  println(nativeInfoLocalBound("local"))
  println(nativeInfoDependent(box))
  println(nativeInfoClass(19))
  println(nativeInfoClass("nested"))
  println(NativeInfoNumbers.direct(23))
  println(NativeInfoNumbers.poly(29))
  println(NativeInfoWords.direct("owner"))
  println(NativeInfoWords.poly("poly"))
  println(nativeInfoNested(31))
  println(nativeInfoNested("inner"))
  println(nativeInfoCarriers(5))
