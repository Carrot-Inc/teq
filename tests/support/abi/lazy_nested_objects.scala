// The numbers of lazy names through objects nested in objects reached statically, which are no fields of their
// object: each nested object at its place in the template (MegaPhase meets its module val and class there), before
// the enclosing object's own later lazy val; their handles looked up in their own `<clinit>`.
// abi: x$lzy1 x$lzy1$lzyHandle x$lzyINIT1 x$lzy2 x$lzy2$lzyHandle x$lzyINIT2 x$lzy3 x$lzy3$lzyHandle x$lzyINIT3 x$lzy4 x$lzy4$lzyHandle x$lzyINIT4 <clinit>
package lazy_nested_objects

object A {
  object B { lazy val x: Int = 1 }
  lazy val x: Int = 2
  object C { lazy val x: Int = 3; object D { lazy val x: Int = 4 } }
}
