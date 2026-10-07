---
title: "重いページの見本：数式の多い記事"
slug: heavy-math
date: 2026-09-26
description: 表示の重さを確かめるための見本。微分積分、線形代数、確率と統計、数論、物理の公式を、文中の数式とブロックの数式で 100 個ほど並べる。
tags: [見本, 計測]
---

:::message
このページは、表示の重さを確かめるための見本です。
数式が多い記事で、KaTeX の読み込みと数式の組版が、本文の表示や入力への応答をどれだけ遅らせるかを Lighthouse で計測しています。
:::

よく知られた公式を、分野ごとに並べます。
文中の数式は `$` で、ブロックの数式は `$$` で囲んで書いています。

## 微分

関数 $f$ の $x$ での微分は、極限で定めます。

$$
f'(x) = \lim_{h \to 0} \frac{f(x + h) - f(x)}{h}
$$

積の微分と商の微分は次のとおりです。

$$
(fg)' = f'g + fg', \qquad \left(\frac{f}{g}\right)' = \frac{f'g - fg'}{g^2}
$$

合成関数の微分（連鎖律）は、$y = f(u)$、$u = g(x)$ のとき次のように書けます。

$$
\frac{dy}{dx} = \frac{dy}{du} \cdot \frac{du}{dx}
$$

よく使う関数の微分を表にまとめます。

| 関数 | 導関数 |
|---|---|
| $x^n$ | $n x^{n-1}$ |
| $e^x$ | $e^x$ |
| $\log x$ | $\dfrac{1}{x}$ |
| $\sin x$ | $\cos x$ |
| $\cos x$ | $-\sin x$ |
| $\tan x$ | $\dfrac{1}{\cos^2 x}$ |

$x = a$ のまわりでのテイラー展開は次のとおりです。

$$
f(x) = \sum_{n=0}^{\infty} \frac{f^{(n)}(a)}{n!} (x - a)^n
$$

特に、$e^x$、$\sin x$、$\cos x$ のマクローリン展開は次のようになります。

$$
\begin{aligned}
e^x &= 1 + x + \frac{x^2}{2!} + \frac{x^3}{3!} + \cdots \\
\sin x &= x - \frac{x^3}{3!} + \frac{x^5}{5!} - \cdots \\
\cos x &= 1 - \frac{x^2}{2!} + \frac{x^4}{4!} - \cdots
\end{aligned}
$$

ここから、オイラーの公式 $e^{i\theta} = \cos\theta + i\sin\theta$ が得られ、$\theta = \pi$ とすると $e^{i\pi} + 1 = 0$ です。

## 積分

微分積分学の基本定理は、$F' = f$ のとき次を言います。

$$
\int_a^b f(x)\,dx = F(b) - F(a)
$$

部分積分と置換積分は次のとおりです。

$$
\int_a^b f(x) g'(x)\,dx = \Big[ f(x) g(x) \Big]_a^b - \int_a^b f'(x) g(x)\,dx
$$

$$
\int_a^b f(g(t))\, g'(t)\,dt = \int_{g(a)}^{g(b)} f(x)\,dx
$$

ガウス積分は、二重積分を極座標に直して求めます。

$$
\left( \int_{-\infty}^{\infty} e^{-x^2} dx \right)^2
= \int_0^{2\pi} \int_0^{\infty} e^{-r^2} r \,dr\,d\theta
= \pi
$$

したがって $\displaystyle \int_{-\infty}^{\infty} e^{-x^2} dx = \sqrt{\pi}$ です。

ガンマ関数は $\Gamma(s) = \int_0^\infty t^{s-1} e^{-t}\,dt$ で定め、$\Gamma(n + 1) = n!$、$\Gamma\left(\tfrac{1}{2}\right) = \sqrt{\pi}$ を満たします。

## 級数

等比級数は、$|r| < 1$ のとき収束します。

$$
\sum_{n=0}^{\infty} r^n = \frac{1}{1 - r}
$$

バーゼル問題の答えと、ライプニッツの級数です。

$$
\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}, \qquad
\sum_{n=0}^{\infty} \frac{(-1)^n}{2n + 1} = \frac{\pi}{4}
$$

調和級数 $\sum 1/n$ は発散しますが、部分和と対数の差は収束します。

$$
\gamma = \lim_{n \to \infty} \left( \sum_{k=1}^{n} \frac{1}{k} - \log n \right) \approx 0.5772
$$

二項定理は次のとおりです。

$$
(x + y)^n = \sum_{k=0}^{n} \binom{n}{k} x^{n-k} y^k, \qquad \binom{n}{k} = \frac{n!}{k!\,(n-k)!}
$$

## 線形代数

$2 \times 2$ の行列の行列式と逆行列です。

$$
A = \begin{pmatrix} a & b \\ c & d \end{pmatrix}, \qquad
\det A = ad - bc, \qquad
A^{-1} = \frac{1}{ad - bc} \begin{pmatrix} d & -b \\ -c & a \end{pmatrix}
$$

$n$ 次の行列式は、置換 $\sigma$ の符号 $\operatorname{sgn}\sigma$ を使って定めます。

$$
\det A = \sum_{\sigma \in S_n} \operatorname{sgn}(\sigma) \prod_{i=1}^{n} a_{i,\sigma(i)}
$$

固有値 $\lambda$ と固有ベクトル $\mathbf{v} \neq \mathbf{0}$ は $A\mathbf{v} = \lambda \mathbf{v}$ を満たし、固有値は特性方程式 $\det(A - \lambda I) = 0$ の解です。

回転の行列は次のとおりです。

$$
R(\theta) = \begin{pmatrix} \cos\theta & -\sin\theta \\ \sin\theta & \cos\theta \end{pmatrix}
$$

連立一次方程式を行列で書きます。

$$
\begin{cases}
a_{11} x_1 + a_{12} x_2 + \cdots + a_{1n} x_n = b_1 \\
a_{21} x_1 + a_{22} x_2 + \cdots + a_{2n} x_n = b_2 \\
\quad \vdots \\
a_{m1} x_1 + a_{m2} x_2 + \cdots + a_{mn} x_n = b_m
\end{cases}
\iff
\begin{pmatrix}
a_{11} & \cdots & a_{1n} \\
\vdots & \ddots & \vdots \\
a_{m1} & \cdots & a_{mn}
\end{pmatrix}
\begin{pmatrix} x_1 \\ \vdots \\ x_n \end{pmatrix}
=
\begin{pmatrix} b_1 \\ \vdots \\ b_m \end{pmatrix}
$$

内積とコーシー＝シュワルツの不等式です。

$$
\langle \mathbf{u}, \mathbf{v} \rangle = \sum_{i=1}^{n} u_i v_i, \qquad
|\langle \mathbf{u}, \mathbf{v} \rangle| \le \|\mathbf{u}\| \, \|\mathbf{v}\|
$$

## 確率と統計

事象 $A$、$B$ について、条件付き確率とベイズの定理は次のとおりです。

$$
P(A \mid B) = \frac{P(A \cap B)}{P(B)}, \qquad
P(A \mid B) = \frac{P(B \mid A)\, P(A)}{P(B)}
$$

確率変数 $X$ の期待値と分散です。

$$
E[X] = \sum_{x} x\, P(X = x), \qquad
V[X] = E\left[(X - E[X])^2\right] = E[X^2] - E[X]^2
$$

代表的な分布を表にまとめます。

| 分布 | 確率（密度）関数 | 期待値 | 分散 |
|---|---|---|---|
| 二項分布 | $\binom{n}{k} p^k (1-p)^{n-k}$ | $np$ | $np(1-p)$ |
| ポアソン分布 | $\dfrac{\lambda^k e^{-\lambda}}{k!}$ | $\lambda$ | $\lambda$ |
| 指数分布 | $\lambda e^{-\lambda x}$ | $\dfrac{1}{\lambda}$ | $\dfrac{1}{\lambda^2}$ |
| 一様分布 | $\dfrac{1}{b - a}$ | $\dfrac{a + b}{2}$ | $\dfrac{(b - a)^2}{12}$ |

正規分布の密度関数です。

$$
f(x) = \frac{1}{\sqrt{2\pi\sigma^2}} \exp\left( -\frac{(x - \mu)^2}{2\sigma^2} \right)
$$

中心極限定理：独立で同じ分布に従う $X_1, \dots, X_n$ の平均 $\bar{X}_n$ について、次が成り立ちます。

$$
\frac{\bar{X}_n - \mu}{\sigma / \sqrt{n}} \xrightarrow{d} N(0, 1) \quad (n \to \infty)
$$

標本の平均と不偏分散は次のとおりです。

$$
\bar{x} = \frac{1}{n} \sum_{i=1}^{n} x_i, \qquad
s^2 = \frac{1}{n - 1} \sum_{i=1}^{n} (x_i - \bar{x})^2
$$

## 数論

$a$ と $n$ が互いに素のとき、オイラーの定理 $a^{\varphi(n)} \equiv 1 \pmod{n}$ が成り立ちます。
$p$ が素数なら $\varphi(p) = p - 1$ なので、フェルマーの小定理 $a^{p-1} \equiv 1 \pmod{p}$ になります。

オイラーの $\varphi$ 関数は、$n$ の素因数分解から求められます。

$$
\varphi(n) = n \prod_{p \mid n} \left( 1 - \frac{1}{p} \right)
$$

ゼータ関数のオイラー積です。

$$
\zeta(s) = \sum_{n=1}^{\infty} \frac{1}{n^s} = \prod_{p\ \text{は素数}} \frac{1}{1 - p^{-s}} \qquad (\operatorname{Re} s > 1)
$$

素数定理は、$x$ 以下の素数の個数 $\pi(x)$ について次を言います。

$$
\pi(x) \sim \frac{x}{\log x} \quad (x \to \infty)
$$

## 物理

ニュートンの運動方程式 $\mathbf{F} = m\mathbf{a}$ と、万有引力の法則です。

$$
F = G \frac{m_1 m_2}{r^2}
$$

単振動の方程式と、その解です。

$$
m \frac{d^2 x}{dt^2} = -kx, \qquad x(t) = A \cos(\omega t + \phi), \qquad \omega = \sqrt{\frac{k}{m}}
$$

真空中のマクスウェル方程式です。

$$
\begin{aligned}
\nabla \cdot \mathbf{E} &= \frac{\rho}{\varepsilon_0} &
\nabla \cdot \mathbf{B} &= 0 \\
\nabla \times \mathbf{E} &= -\frac{\partial \mathbf{B}}{\partial t} &
\nabla \times \mathbf{B} &= \mu_0 \mathbf{J} + \mu_0 \varepsilon_0 \frac{\partial \mathbf{E}}{\partial t}
\end{aligned}
$$

シュレーディンガー方程式です。

$$
i\hbar \frac{\partial}{\partial t} \Psi(\mathbf{r}, t) = \left( -\frac{\hbar^2}{2m} \nabla^2 + V(\mathbf{r}, t) \right) \Psi(\mathbf{r}, t)
$$

特殊相対論の質量とエネルギーの関係 $E = mc^2$ は、運動量 $p$ を持つ粒子では $E^2 = (pc)^2 + (mc^2)^2$ になります。
ローレンツ因子は $\gamma = 1 / \sqrt{1 - v^2 / c^2}$ です。

理想気体の状態方程式 $pV = nRT$ と、熱力学の第一法則 $dU = \delta Q - \delta W$ も、よく使います。

## 書き誤りのある数式

KaTeX が読めない数式は、TeX の文字列のまま表示します。

$$
\frac{1}{
$$
