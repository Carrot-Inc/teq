object U0:
  def make(): BasicQueue = new BasicQueue with Incrementing0 with Filtering0

trait Incrementing0 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 0)

trait Filtering0 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 0 then super.put(x)
