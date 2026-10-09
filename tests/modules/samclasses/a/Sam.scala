package saa

// The classes SAM conversions make, among anonymous classes of their file: named and ordered as
// the whole program has them.
trait Op:
  def run(x: Int): Int

trait Label:
  def text(n: Int): String

object Ops:
  val twice: Op = (x: Int) => x * 2
  val plain: Op = new Op:
    def run(x: Int): Int = x + 1
  def offset(k: Int): Op = (x: Int) => x + k
  val label: Label = n => "n=" + n

class Pipeline(ops: List[Op]):
  def apply(x: Int): Int = ops.foldLeft(x)((acc, op) => op.run(acc))
  def tagged: Label = n => "pipeline " + apply(n)
