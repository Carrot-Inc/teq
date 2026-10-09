// expect: yclic reference involving
// An import through a val of the class's instance whose initialiser makes a class the import
// is read in the completion of is a cyclic reference (E046). scalac reports it at the import as
// one involving the val; teq where the initialiser constructs the class, so only the words both
// share are pinned.
class C:
  val a = new D(1)
  import a.*
  class D(val n: Int):
    def m: Int = n + 1
  def read: Int = m
