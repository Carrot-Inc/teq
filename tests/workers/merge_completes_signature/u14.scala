object U14:
  def make(): BasicQueue = new BasicQueue with Incrementing14 with Filtering14

trait Incrementing14 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 14)

trait Filtering14 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 14 then super.put(x)
