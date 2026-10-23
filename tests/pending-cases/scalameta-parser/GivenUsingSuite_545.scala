// Program 545 of tests/fix/scalameta, from scalameta 4.17.4's GivenUsingSuite (line 466, "given without sig and type
// in parens looking like a param clause"): scalac 3.8.4's parser accepts it, teq's reports "expected end of
// statement, found identifier". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree
// answers to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 91
//     Defn.Object 0 90
//       Term.Name 7 11
//       Template 12 90
//         Template.Body 12 90
//           Defn.Def 16 88
//             Term.Name 20 21
//             Term.Block 24 88
//               Defn.Given 26 85
//                 Name.Anonymous 32 32
//                 Template 32 85
//                   Init 32 47
//                     Type.Apply 33 46
//                       Type.Name 33 38
//                       Type.ArgClause 38 46
//                         Type.Name 39 45
//                     Name.Anonymous 48 48
//                   Init 53 60
//                     Type.Apply 53 60
//                       Type.Name 53 55
//                       Type.ArgClause 55 60
//                         Type.Name 56 59
//                     Name.Anonymous 61 61
//                   Template.Body 66 85
//                     Defn.Def 70 83
//                       Term.Name 74 77
//                       Term.Name 80 83
object W545 {
  def w = {
given (using[String]) with Eq[Int] with {
  def foo = ???
}

}
}
