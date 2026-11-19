// A compilation with one file at 3.6 (Old.scala) and one at the default (Mine.scala), both reading a given object of
// several parents through the level of the package that defines it. scalac 3.8.4 reads it as `P & Q` in both files
// and prints `3.6: 12; default: 12`. teq reads every file of a compilation with a file at 3.6 or 3.7-migration as it
// did before it told the versions apart (the first parent, `P`), and prints `3.6: 10; default: 10`, whatever the order
// in which the files are typed, until the part that ports those versions' comparison.
package pk

class P { def x: Int = 1 }
object P:
  given fallbackP: P = new P { override def x: Int = 0 }
trait Q { def q: Int = 2 }
object Q:
  given fallbackQ: Q = new Q { override def q: Int = 0 }

given g: P() with Q with {}
