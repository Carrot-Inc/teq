// dotty's tests/neg/export-in-extension.scala: an export clause inside an extension group names a companion
// extension method of the group (`Namer.exportPathSym`), its forwarders extension methods over the method's
// parameters (`exportForwarders` with `pathMethod`); a type is no such forwarder's, nor a member of a block.
// The clause `export this.cm.baz` of dotty's file is left out: a `this` path is the import parser's.
// expect: 26:14: error: export qualifier c1 is not a parameterless companion extension method
// expect: 31:23: error: no eligible member D at O.O2.cm
// expect: O.O2.cm.D cannot be exported because it is a type, so it cannot be exported as extension method
// expect: 36:14: error: export qualifier missing is not a parameterless companion extension method
// expect: 40:14: error: export qualifier cm is not a parameterless companion extension method
// expect: 45:14: error: export qualifier cm is not a parameterless companion extension method
// expect: 50:7: error: exports are only allowed from objects and classes, they can not belong to local blocks
// expect: 6 errors found
object O:

  class C(x: Int):
    def bar = x
    def baz(y: Int) = x + y
    val bam = x * x
    def :: (y: Int) = x - y
    class D

  val c1 = C(1)

  object O1:
    extension (x: Int)
      export c1.*   // error

  object O2:
    extension (x: Int)
      private def cm = new C(x)
      export cm.{bar, D}  // error
      // export this.cm.baz  // error in dotty's file: a `this` path (see the header)

  object O3:
    extension (x: Int)
      export missing.*  // error

  object O4:
    extension (x: Int)
      export cm.*   // error

  object O5:
    extension (x: Int)
      private def cm(y: C) = y
      export cm.*   // error

  {
    extension (x: Int)
      private def cm = new C(x)
      export cm.*   // error
  }
