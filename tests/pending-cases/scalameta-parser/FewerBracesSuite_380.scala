// Program 380 of tests/fix/scalameta, from scalameta 4.17.4's FewerBracesSuite (line 1693, "scalafmt #3763 using
// after newline, with type"): scalac 3.8.4's parser accepts it, teq's reports "expected end of statement, found
// identifier". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a
// line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 62
//     Defn.Object 0 61
//       Term.Name 7 11
//       Template 12 61
//         Template.Body 12 61
//           Defn.Def 15 58
//             Term.Name 19 20
//             Term.Apply 23 58
//               Term.Name 23 26
//               Term.ArgClause 26 58
//                 Term.Block 26 58
//                   Term.Function 31 58
//                     Term.ParamClause 31 45
//                       Term.Param 31 45
//                         Mod.Using 31 36
//                         Term.Name 37 40
//                         Type.Name 42 45
//                       Mod.Using 31 36
//                     Term.Name 55 58
object W380 {

def a = foo:
   using bar: Int =>
      baz

}
