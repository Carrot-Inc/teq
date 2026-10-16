// A trait's clone of a generic result, mixed in at String, at the class's own type parameter, and of an
// array of it: the forwarders are raw, as every method teq writes is (no generic signature yet), and a
// Java client reads them so, by javac and by reflection.
package genericclone

trait G[A]:
  override def clone(): java.util.List[A] = new java.util.ArrayList[A]()

trait ArrayG[A <: AnyRef]:
  override def clone(): Array[A] = null
