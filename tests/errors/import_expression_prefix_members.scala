// An import's qualifier that is no stable path is typed as an expression (`typedImportQualifier`)
// and its selectors are checked against the tree's type (`Checking.checkImportSelectors`'s
// `qualType.member(name)`): a method's result, a transparent inline method's expansion, whose
// type the stability check names (`C is not a valid import prefix`) in the place of a path.
// expect: 13:10: error: (Test.p : => C) is not a valid import prefix, since it is not an immutable path
// expect: 13:12: error: value noSuchMember is not a member of C
// expect: 15:10: error: C is not a valid import prefix, since it is not an immutable path
// expect: 15:12: error: value noSuchMember is not a member of C
// expect: 17:10: error: (Test.v : C) is not a valid import prefix, since it is not an immutable path
class C { val x: Int = 1 }
object Test {
  def p: C = new C
  import p.noSuchMember
  transparent inline def q: C = new C
  import q.noSuchMember
  var v: C = new C
  import v.x
}
