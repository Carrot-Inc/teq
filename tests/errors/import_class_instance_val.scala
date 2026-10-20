// An import's prefix is a stable path (dotty's `Checking.checkStable`, E083): a method or a var
// of the enclosing class's instance is none, where its vals are; a lazy val that is not final is no
// legal one (`checkLegalImportOrExportPath`); a private member is no member an import brings; a
// named selector is a member of the value's type, read or not (`Checking.checkImportSelectors`).
// The qualifier is typed as an expression (`typedImportQualifier`), its `TermRef` shown with the
// enclosing instance (`Use.this.d`), and every check of `checkLegalImportOrExportPath` reports:
// the realizability of a method or a var of an extensible class (`NotFinal`) and the purity of the
// reference beside the stability, which scalac's reporter hides at the one position
// (`UniqueMessagePositions`), as `import_prefixless` notes.
// expect: import_class_instance_val.scala:26:10: error: (Use.this.d : => API) is not a valid import prefix, since it is not an immutable path
// expect: import_class_instance_val.scala:26:10: error: (Use.this.d : => API) is not a legal path since it refers to nonfinal method d
// expect: import_class_instance_val.scala:26:10: error: import prefix is not a pure expression
// expect: import_class_instance_val.scala:28:10: error: (Use.this.v : API) is not a valid import prefix, since it is not an immutable path
// expect: import_class_instance_val.scala:28:10: error: (Use.this.v : API) is not a legal path since it refers to nonfinal variable v
// expect: import_class_instance_val.scala:28:10: error: import prefix is not a pure expression
// expect: import_class_instance_val.scala:30:10: error: (Use.this.l : API) is not a legal path since it refers to nonfinal lazy value l
// expect: import_class_instance_val.scala:33:19: error: not found: hidden
// expect: import_class_instance_val.scala:35:12: error: value noSuch is not a member of API
// expect: import_class_instance_val.scala:36:13: error: value absent is not a member of API
// expect: import_class_instance_val.scala:40:12: error: value gone is not a member of API
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
