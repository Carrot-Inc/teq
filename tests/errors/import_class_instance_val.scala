// expect: import_class_instance_val.scala:17:10: error: (Use.this.d : => API) is not a valid import prefix, since it is not an immutable path
// expect: import_class_instance_val.scala:19:10: error: (Use.this.v : API) is not a valid import prefix, since it is not an immutable path
// expect: import_class_instance_val.scala:21:10: error: (Use.this.l : API) is not a legal path since it refers to nonfinal lazy value l
// expect: import_class_instance_val.scala:24:19: error: not found: hidden
// expect: import_class_instance_val.scala:26:12: error: value noSuch is not a member of API
// expect: import_class_instance_val.scala:27:13: error: value absent is not a member of API
// expect: import_class_instance_val.scala:31:12: error: value gone is not a member of API
// An import's prefix is a stable path (dotty's `Checking.checkStable`, E083): a method or a var
// of the enclosing class's instance is none, where its vals are; a lazy val that is not final is no
// legal one (`checkLegalImportOrExportPath`); a private member is no member an import brings; a
// named selector is a member of the value's type, read or not (`Checking.checkImportSelectors`).
class API:
  val value = 7
  private val hidden = 8
class Use:
  def d = new API
  import d.*
  var v = new API
  import v.*
  lazy val l = new API
  import l.*
class Named(val a: API):
  import a.hidden
  def read: Int = hidden
class Missing(val a: API):
  import a.noSuch
  import a.{absent => renamed, value}
  def read: Int = value
def local(): Int =
  val a = new API
  import a.gone
  a.value
