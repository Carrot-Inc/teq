object U1:
  def make(): BasicQueue = new BasicQueue with Incrementing1 with Filtering1

trait Incrementing1 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 1)

trait Filtering1 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 1 then super.put(x)
