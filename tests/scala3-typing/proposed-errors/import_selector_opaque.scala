// expect: value noSuchMember is not a member of O.Id
// scalac 3.8.4: -- [E008] Not Found Error: Test.scala:2:26 ... value noSuchMember is not a member of O.Id
// (`Checking.checkImportSelectors` over the opaque type's members as seen outside its scope; teq exempts it).
// The review of 5010f41f8, finding 3; the queued part inlining-lila-folds.
object O { opaque type Id = Int; val id: Id = 1 }
object Test { import O.id.noSuchMember }
