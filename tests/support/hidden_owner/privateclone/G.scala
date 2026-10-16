// A trait's private clone, which no class mixing it in forwards (`needsMixinForwarder` excludes a private
// member), from its source as from its products, so that `Object`'s protected clone is the class's to Java.
package privateclone

trait G:
  private def clone[A](): String = "private"
  def f(): String = clone[Int]()
