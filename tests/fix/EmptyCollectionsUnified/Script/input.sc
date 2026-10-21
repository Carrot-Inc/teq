/*
rules = EmptyCollectionsUnified
*/
// A script is edited as written (the review of `86884ea36`): the wrapper teq types it in, appended past its text, is
// no part of its document, so the rewrite writes `List.empty` and nothing of the wrapper.
val xs = List()
println(xs)
