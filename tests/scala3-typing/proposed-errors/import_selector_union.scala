// expect: value noSuchMember is not a member of Int | String
// scalac 3.8.4: -- [E008] Not Found Error: Test.scala:1:48 ... value noSuchMember is not a member of Int | String
// (`Checking.checkImportSelectors`' `qualType.member(sel.name)` over the union; teq exempts a non-class prefix shape).
// The review of 5010f41f8, finding 3; the queued part inlining-lila-folds.
object Test { val n: Int | String = 1; import n.noSuchMember }
