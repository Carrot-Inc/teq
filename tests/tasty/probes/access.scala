// Qualified access of classes and objects: `private[p]` and `protected[p]` inside an object,
// a private class of an object, a private top-level class (`private[p]` of its package).
package pq

object A:
  private[pq] class C:
    def f: Int = 42
  protected[pq] class D
  private class E

private class Top
private[pq] class Top2
