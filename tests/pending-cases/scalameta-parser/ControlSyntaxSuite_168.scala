// Program 168 of tests/fix/scalameta, from scalameta 4.17.4's ControlSyntaxSuite (line 3966, "scalafmt #3790 match
// optional braces, and case class following after case body"): scalac 3.8.4's parser accepts it, teq's reports
// "expected a pattern, found 'class'". The expectation is scalameta's tree under dialects.Scala3, which teq fix's
// tree answers to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 100
//     Defn.Object 0 99
//       Term.Name 7 11
//       Template 12 99
//         Template.Body 12 99
//           Defn.Def 14 96
//             Term.Name 18 21
//             Term.Block 26 96
//               Term.Match 26 73
//                 Term.Name 26 29
//                 Term.CasesBlock 38 73
//                   Case 38 73
//                     Lit.Int 43 44
//                     Term.Block 52 73
//                       Term.Apply 52 62
//                         Term.Name 52 59
//                         Term.ArgClause 59 62
//                           Lit.Int 60 61
//                       Term.Tuple 67 73
//                         Lit.Int 68 69
//                         Lit.Int 71 72
//               Defn.Class 76 96
//                 Mod.Case 76 80
//                 Type.Name 87 88
//                 Type.ParamClause 88 88
//                 Ctor.Primary 88 96
//                   Name.Anonymous 88 88
//                   Term.ParamClause 88 96
//                     Term.Param 89 95
//                       Term.Name 89 90
//                       Type.Name 92 95
//                 Template 97 97
//                   Template.Body 97 97
object W168 {
def foo =
  bar match
  case 1 =>
    println(2)
    (3, 4)
  case class A(a: Int)

}
