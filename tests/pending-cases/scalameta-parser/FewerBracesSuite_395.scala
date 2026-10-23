// Program 395 of tests/fix/scalameta, from scalameta 4.17.4's FewerBracesSuite (line 2034, "scalafmt #3763 erased
// after newline, with type"): scalac 3.8.4's parser accepts it, teq's reports "expected end of statement, found
// identifier". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a
// line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 63
//     Defn.Object 0 62
//       Term.Name 7 11
//       Template 12 62
//         Template.Body 12 62
//           Defn.Def 15 59
//             Term.Name 19 20
//             Term.Apply 23 59
//               Term.Name 23 26
//               Term.ArgClause 26 59
//                 Term.Block 26 59
//                   Term.Function 31 59
//                     Term.ParamClause 31 46
//                       Term.Param 31 46
//                         Mod.Erased 31 37
//                         Term.Name 38 41
//                         Type.Name 43 46
//                     Term.Name 56 59
object W395 {

def a = foo:
   erased bar: Int =>
      baz

}
