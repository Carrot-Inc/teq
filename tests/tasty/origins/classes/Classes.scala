package oc

// The classes bodies define, whose generated names the origins record: an anonymous class of a
// method, one nested in another, one of a top-level definition, a named local class.
trait Shown:
  def show: String

object Classes:
  def anonymous(n: Int): Shown = new Shown:
    def show: String = "n " + n
  def nested: Shown = new Shown:
    def show: String = new Shown { def show = "inner" }.show
  def local(k: Int): Int =
    class Twice(val v: Int):
      def get: Int = v * 2
    new Twice(k).get

def topLevel: Shown = new Shown:
  def show: String = "top"
