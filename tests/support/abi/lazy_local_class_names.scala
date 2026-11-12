// The numbers of lazy names as dotty's MegaPhase meets the definitions (FreshNameCreator per file): every local class
// at its declaration, constructed or not, in declaration order, its lazy vals numbered there (an unused class before
// the class's own lazy val; one in a nested block; one constructed only after a later local lazy val); a local class
// of a top-level def after the top-level classes, the package object coming last (Desugar.packageDef). The members of
// the top-level classes are listed: the local classes' binary names are each compiler's own.
// abi: x$lzy2 x$lzy2$lzyHandle x$lzyINIT2 y$lzy1 y$lzy1$lzyHandle y$lzyINIT1 z$lzy5 z$lzy5$lzyHandle z$lzyINIT5
package lazy_local_class_names

class A {
  def go(): Unit = { class Local { lazy val x: Int = 1 }; () }
  lazy val x: Int = 2
}

def topLevel(): Int = { class TopLocal { lazy val y: Int = 3 }; 0 }

class B {
  lazy val y: Int = 4
}

class C {
  def go(): Int = {
    val a = 1
    class First { lazy val z: Int = 5 }
    lazy val z: Int = 6
    if a > 0 then { class Nested { lazy val z: Int = 7 }; () }
    class Last { lazy val z: Int = 8 }
    z + new Last().z
  }
  lazy val z: Int = 9
}
