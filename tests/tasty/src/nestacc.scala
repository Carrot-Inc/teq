package fix.nestacc

// An inline method of a class nested in objects that reads its class's private members: scalac's inline accessors
// `inline$fix$nestacc$AccNest$Range$$end` and `inline$fix$nestacc$AccNest$Inner$Deep$$k`, each object's segment its
// module's name (`fullNameSeparated` drops a prefix's `ModuleClassName` under the `$` separator).
object AccNest:
  class Range(from: Int, end: Int):
    inline def foreach(op: Int => Unit): Unit =
      var i = from
      while i < end do
        op(i)
        i += 1
  object Inner:
    class Deep(private val k: Int):
      inline def get: Int = k
