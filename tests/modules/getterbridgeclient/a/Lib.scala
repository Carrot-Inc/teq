//> using options -Xmixin-force-forwarders:false

package gbl

// A class inheriting a trait's default getter of a value class's type, where the root's getter erases to Object: its
// class file holds the getter's bridge `m$default$1()Ljava/lang/Object;` and no forwarder of the getter (teq's class
// form, and scalac's without forced mixin forwarders, which the directive asks of the over-scalac leg's build), which
// a client over the class files reads as no declaration of the getter (dotty's class-file reader enters no synthetic
// member: ClassfileParser.parseMember).
class V(val x: Int) extends AnyVal { override def toString: String = "V" + x }
trait A[X] { def m(a: X = null.asInstanceOf[X]): String = "A" + a }
trait B extends A[V] { override def m(a: V = new V(2)): String = "B" + a }
class P extends B { override def m(a: V): String = "P" + a }
