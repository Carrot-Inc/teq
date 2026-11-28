// A private member selected on another instance of its class is `private`, not `private[this]`: dotty's Namer
// makes every private member Local, and an access through a prefix other than `this` takes Local off
// (`SymDenotations.isAccessibleFrom`'s `resetFlag(Local)`), off its default's getter too. teq records no such
// access and writes them `private[this]` (C's lines in tests/analysis/pending.txt); D's members, reached through
// `this` alone, are `private[this]` in both.
package pother

class C:
  private def m(x: Int = 1): Int = x
  private val v: Int = 2
  def other(c: C): Int = c.m() + c.v
  def self: Int = m() + v

class D:
  private def m(x: Int = 1): Int = x
  private val v: Int = 2
  def self: Int = m() + v
