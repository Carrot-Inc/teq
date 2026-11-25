// `super.m()` of a member a Java superclass inherits from an interface the std declares it on (an ArrayList's
// toString, hashCode and equals, which the std models on Collection and List) calls the superclass, as scalac names
// it (`invokespecial java/util/ArrayList.toString`): the JDK's AbstractCollection and AbstractList bodies run. A
// trait's `super.toString()` mixed into such a class reaches them the same way, through the trait's accessor; a
// second trait's reaches the first's.
trait T extends java.util.RandomAccess:
  override def toString(): String = "t:" + super.toString()

trait U extends java.util.RandomAccess:
  override def toString(): String = "u:" + super.toString()

class A extends java.util.ArrayList[String]:
  override def toString(): String = "a:" + super.toString()
  override def hashCode(): Int = super.hashCode() + 1
  override def equals(o: Any): Boolean = super.equals(o)

class B extends java.util.ArrayList[String] with T

class C extends java.util.ArrayList[String] with T with U

object Main:
  def main(args: Array[String]): Unit =
    val a = new A
    a.add("x")
    println(a.toString())
    println(a.hashCode())
    val b = new java.util.ArrayList[String]
    b.add("x")
    println(a.equals(b))
    val tb = new B
    tb.add("x")
    println(tb.toString())
    val tc = new C
    tc.add("y")
    println(tc.toString())
