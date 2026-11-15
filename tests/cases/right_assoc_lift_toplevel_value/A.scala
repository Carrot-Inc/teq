// A top-level value is no receiver whose evaluation does nothing: its first read runs its file's initializers,
// from a class's method of the same file and through an inline method another file expands as from anywhere, so
// the operand is bound ahead of it. Master read the value first in both.
package one
val items: List[Int] = { println("init one"); List(1) }
class C { def f(): List[Int] = main.Ext.left() :: items }
