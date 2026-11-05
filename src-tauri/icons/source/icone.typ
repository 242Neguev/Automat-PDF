// Source de l'icône de l'application, aux couleurs de l'éditeur Eau sèche
// (Encre #22252A, Eau #1E6E72 en accent unique, Sec #EFEDE8). Régénérer les icônes :
//   typst compile src-tauri/icons/source/icone.typ icone.png --ppi 72 && npx tauri icon icone.png
#set page(width: 1024pt, height: 1024pt, margin: 0pt, fill: none)
#let encre = rgb("#22252A")
#let eau = rgb("#1E6E72")
#let sec = rgb("#EFEDE8")
#let pli = rgb("#C9C5BA")
// Une page au coin plié ; le contour arrondit les angles.
#let page-pliee(couleur, largeur, hauteur, coin) = polygon(
  fill: couleur,
  stroke: (paint: couleur, thickness: 44pt, join: "round"),
  (0pt, 0pt), (largeur - coin, 0pt), (largeur, coin), (largeur, hauteur), (0pt, hauteur),
)
#place(center + horizon, rect(width: 904pt, height: 904pt, radius: 210pt, fill: encre))
#place(top + left, dx: 330pt, dy: 190pt, page-pliee(eau, 420pt, 560pt, 130pt))
#place(top + left, dx: 262pt, dy: 268pt, page-pliee(sec, 420pt, 560pt, 130pt))
#place(top + left, dx: 262pt + 290pt, dy: 268pt, polygon(
  fill: pli,
  stroke: (paint: pli, thickness: 44pt, join: "round"),
  (0pt, 0pt), (0pt, 130pt), (130pt, 130pt),
))
#place(top + left, dx: 262pt, dy: 268pt + 300pt, box(width: 420pt, align(center, text(font: "Arial", weight: "bold", size: 170pt, fill: encre, tracking: -4pt)[PDF])))
