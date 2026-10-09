package p

import q.*

// A definition of the same file beats the wildcard import; a definition of another file of the
// package loses to it.
class Foo:
  override def toString = "p.Foo"

object Show:
  def foo = new Foo
  def bar = new Bar
