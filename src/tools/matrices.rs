//! Matrix algebra — multiplication, inverse, determinant, decomposition.
//!
//! Matrices are passed as JSON-style nested CSV: `"1,2;3,4"` for a 2x2 matrix
//! (rows separated by `;`, columns by `,`). All values are `f64`. For exact
//! arithmetic on small matrices, callers can compose with `evaluateExact`.

use crate::mcp::message::{ErrorCode, Response, error, error_with_detail};
use crate::tools::numeric::{canonicalize_zero, snap_to_precision};

const TOOL_MATRIX_ADD: &str = "MATRIX_ADD";
const TOOL_MATRIX_MULT: &str = "MATRIX_MULT";
const TOOL_MATRIX_TRANSPOSE: &str = "MATRIX_TRANSPOSE";
const TOOL_MATRIX_DETERMINANT: &str = "MATRIX_DETERMINANT";
const TOOL_MATRIX_INVERSE: &str = "MATRIX_INVERSE";
const TOOL_MATRIX_TRACE: &str = "MATRIX_TRACE";
const TOOL_MATRIX_RANK: &str = "MATRIX_RANK";
const TOOL_MATRIX_EIGENVALUES_2X2: &str = "MATRIX_EIGENVALUES_2X2";
const TOOL_CROSS_PRODUCT: &str = "CROSS_PRODUCT";
const TOOL_GAUSSIAN_ELIMINATION: &str = "GAUSSIAN_ELIMINATION";

#[derive(Clone)]
struct Matrix {
    rows: usize,
    cols: usize,
    data: Vec<Vec<f64>>,
}

impl Matrix {
    fn parse(tool: &str, label: &str, input: &str) -> Result<Self, String> {
        let rows: Vec<&str> = input.split(';').filter(|s| !s.trim().is_empty()).collect();
        if rows.is_empty() {
            return Err(error_with_detail(
                tool,
                ErrorCode::InvalidInput,
                "matrix has no rows",
                &format!("{label}={input}"),
            ));
        }
        let mut data = Vec::with_capacity(rows.len());
        let mut cols = 0;
        for (i, row) in rows.iter().enumerate() {
            let cells: Vec<&str> = row.split(',').collect();
            let mut parsed = Vec::with_capacity(cells.len());
            for cell in cells {
                let trimmed = cell.trim();
                match trimmed.parse::<f64>() {
                    Ok(v) if v.is_finite() => parsed.push(v),
                    _ => {
                        return Err(error_with_detail(
                            tool,
                            ErrorCode::ParseError,
                            "matrix cell is not a finite number",
                            &format!("{label} cell={trimmed}"),
                        ));
                    }
                }
            }
            if i == 0 {
                cols = parsed.len();
            } else if parsed.len() != cols {
                return Err(error_with_detail(
                    tool,
                    ErrorCode::InvalidInput,
                    "rows have inconsistent column counts",
                    &format!("row={i}, expected={cols}, got={}", parsed.len()),
                ));
            }
            data.push(parsed);
        }
        Ok(Self {
            rows: rows.len(),
            cols,
            data,
        })
    }

    fn format(&self) -> String {
        self.data
            .iter()
            .map(|row| row.iter().map(|v| fmt(*v)).collect::<Vec<_>>().join(","))
            .collect::<Vec<_>>()
            .join(";")
    }

    /// First non-finite cell position (for OVERFLOW reporting) or `None` when
    /// every cell is a real number. Matrix multiplication / inversion /
    /// gauss-jordan all chain multiplications that can silently saturate to
    /// `±∞`; callers funnel output through here so `MATRIX: inf,inf;inf,inf`
    /// never escapes the envelope.
    fn first_non_finite(&self) -> Option<(usize, usize, f64)> {
        for (i, row) in self.data.iter().enumerate() {
            for (j, &v) in row.iter().enumerate() {
                if !v.is_finite() {
                    return Some((i, j, v));
                }
            }
        }
        None
    }
}

/// Build a matrix `DIM/MATRIX` response, failing with OVERFLOW if the
/// computation produced a non-finite cell.
fn ok_matrix(tool: &str, result: &Matrix) -> String {
    if let Some((i, j, v)) = result.first_non_finite() {
        return error_with_detail(
            tool,
            ErrorCode::Overflow,
            "matrix computation produced a non-finite cell (overflow/underflow to ±∞ or NaN)",
            &format!("row={i}, col={j}, value={v:?}"),
        );
    }
    Response::ok(tool)
        .field("DIM", format!("{}x{}", result.rows, result.cols))
        .field("MATRIX", result.format())
        .build()
}

/// Format a matrix entry, rounding ULP drift away at 15 significant digits.
///
/// Gauss-Jordan and related f64 cascades leak a couple of ULPs even on
/// textbook inputs: the inverse of `[[1,2],[3,4]]` computes to
/// `-1.9999999999999996`, `0.9999999999999998`, `1.4999999999999998`,
/// `-0.49999999999999994` rather than the exact `-2, 1, 1.5, -0.5`. f64 only
/// carries 15–17 significant digits, so rounding at 15 preserves every bit of
/// meaningful information while absorbing the last-digit drift. Truly small
/// values (`1e-10` determinants) survive because the rounding tracks the
/// value's magnitude.
///
/// Also collapses IEEE-754 `-0.0` to `+0.0` so back-substitution chains like
/// `x = (b − a·y − c·z) / d` that legitimately produce `0.0` never surface
/// as user-visible `"-0.0"` — purely a display concern, the bit-pattern
/// difference carries no mathematical meaning here.
fn fmt(value: f64) -> String {
    format!("{:?}", canonicalize_zero(snap_to_precision(value, 15)))
}

#[must_use]
pub fn matrix_add(a: &str, b: &str) -> String {
    let m1 = match Matrix::parse(TOOL_MATRIX_ADD, "a", a) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let m2 = match Matrix::parse(TOOL_MATRIX_ADD, "b", b) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if m1.rows != m2.rows || m1.cols != m2.cols {
        return error_with_detail(
            TOOL_MATRIX_ADD,
            ErrorCode::InvalidInput,
            "matrix dimensions must match",
            &format!("a={}x{}, b={}x{}", m1.rows, m1.cols, m2.rows, m2.cols),
        );
    }
    let mut result = m1.clone();
    for i in 0..m1.rows {
        for j in 0..m1.cols {
            result.data[i][j] += m2.data[i][j];
        }
    }
    ok_matrix(TOOL_MATRIX_ADD, &result)
}

#[must_use]
pub fn matrix_mult(a: &str, b: &str) -> String {
    let m1 = match Matrix::parse(TOOL_MATRIX_MULT, "a", a) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let m2 = match Matrix::parse(TOOL_MATRIX_MULT, "b", b) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if m1.cols != m2.rows {
        return error_with_detail(
            TOOL_MATRIX_MULT,
            ErrorCode::InvalidInput,
            "inner dimensions must match (a.cols == b.rows)",
            &format!("a={}x{}, b={}x{}", m1.rows, m1.cols, m2.rows, m2.cols),
        );
    }
    let m2_cols = m2.cols;
    let m2_data = &m2.data;
    let data: Vec<Vec<f64>> = m1
        .data
        .iter()
        .map(|row_i| {
            (0..m2_cols)
                .map(|j| {
                    row_i
                        .iter()
                        .enumerate()
                        .fold(0.0, |acc, (k, &val_ik)| val_ik.mul_add(m2_data[k][j], acc))
                })
                .collect()
        })
        .collect();
    let result = Matrix {
        rows: m1.rows,
        cols: m2.cols,
        data,
    };
    ok_matrix(TOOL_MATRIX_MULT, &result)
}

#[must_use]
pub fn matrix_transpose(a: &str) -> String {
    let m = match Matrix::parse(TOOL_MATRIX_TRANSPOSE, "a", a) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let rows = m.rows;
    let cols = m.cols;
    let data: Vec<Vec<f64>> = (0..cols)
        .map(|j| m.data.iter().map(|row| row[j]).collect())
        .collect();
    let result = Matrix {
        rows: cols,
        cols: rows,
        data,
    };
    ok_matrix(TOOL_MATRIX_TRANSPOSE, &result)
}

#[must_use]
pub fn matrix_determinant(a: &str) -> String {
    let m = match Matrix::parse(TOOL_MATRIX_DETERMINANT, "a", a) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if m.rows != m.cols {
        return error_with_detail(
            TOOL_MATRIX_DETERMINANT,
            ErrorCode::InvalidInput,
            "determinant requires a square matrix",
            &format!("dim={}x{}", m.rows, m.cols),
        );
    }
    let n = m.rows;
    let col_maxes = column_max_abs(&m.data, n);
    let det_raw = compute_det(m.data);
    // Column-product zero-snap.
    //
    // The determinant of an n×n matrix is bounded by the product of its
    // columns' maxima (`|det(A)| ≤ ∏ max|col_j|` by Hadamard's inequality).
    // Scaling that bound by `n · ε` gives a dimension-aware threshold that
    // catches the drift left on rank-deficient integer matrices
    // (`[[1,2,3],[4,5,6],[7,8,9]]` leaks ~6.66e-16) without rejecting
    // honest determinants of matrices with disparate column magnitudes
    // (`diag(1e200, 1e-200)` has `|det| = 1` and the threshold lands at
    // `~4.4e-16`, so the `1` survives untouched).
    let n_f = num_traits::NumCast::from(n).unwrap_or(1.0_f64);
    let col_product: f64 = col_maxes.iter().product();
    let threshold = n_f * col_product * f64::EPSILON;
    let det = if det_raw.abs() < threshold {
        0.0
    } else {
        det_raw
    };
    if !det.is_finite() {
        return error_with_detail(
            TOOL_MATRIX_DETERMINANT,
            ErrorCode::Overflow,
            "determinant is non-finite (overflow/underflow to ±∞ or NaN)",
            &format!("det={det:?}"),
        );
    }
    Response::ok(TOOL_MATRIX_DETERMINANT)
        .result(fmt(det))
        .build()
}

/// Per-column max absolute value. Helper used by the Hadamard-bound
/// determinant threshold. Returns one value per column in `0..cols`; an
/// all-zero column yields `0.0`.
fn column_max_abs(data: &[Vec<f64>], cols: usize) -> Vec<f64> {
    (0..cols)
        .map(|j| data.iter().map(|row| row[j].abs()).fold(0.0_f64, f64::max))
        .collect()
}

/// LU-style determinant via partial-pivoted Gaussian elimination on a clone.
fn compute_det(mut a: Vec<Vec<f64>>) -> f64 {
    let n = a.len();
    let mut sign = 1.0;
    for i in 0..n {
        // partial pivot — find the row in [i+1, n) with the largest |a[k][i]|.
        let pivot = (i..n)
            .max_by(|&x, &y| {
                a[x][i]
                    .abs()
                    .partial_cmp(&a[y][i].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(i);
        if a[pivot][i] == 0.0 {
            return 0.0;
        }
        if pivot != i {
            a.swap(pivot, i);
            sign = -sign;
        }
        let pivot_value = a[i][i];
        let pivot_row = a[i].clone();
        for row in a.iter_mut().skip(i + 1) {
            let factor = row[i] / pivot_value;
            for (cell, &top) in row.iter_mut().zip(pivot_row.iter()).skip(i) {
                *cell -= factor * top;
            }
        }
    }
    a.iter()
        .enumerate()
        .fold(sign, |acc, (i, row)| acc * row[i])
}

/// One Gauss-Jordan step on the augmented matrix at column `i`. Returns
/// `Ok(())` on success, or `Err(())` if the pivot is effectively zero.
///
/// `zero_threshold` is the **per-column** numeric floor for this column —
/// pivots below it are treated as structurally zero. A single matrix-wide
/// threshold fails for matrices with widely disparate column magnitudes
/// (e.g. `diag(1e200, 1e-200)`) where the global `‖A‖∞` dominates and
/// buries the legitimately small pivot; one threshold per column lets each
/// column contribute to its own singularity test.
fn gauss_jordan_step(
    aug: &mut [Vec<f64>],
    i: usize,
    n: usize,
    zero_threshold: f64,
) -> Result<(), ()> {
    let pivot = (i..n)
        .max_by(|&x, &y| {
            aug[x][i]
                .abs()
                .partial_cmp(&aug[y][i].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(i);
    if aug[pivot][i].abs() < zero_threshold {
        return Err(());
    }
    aug.swap(pivot, i);
    let div = aug[i][i];
    for cell in &mut aug[i] {
        *cell /= div;
    }
    let pivot_row = aug[i].clone();
    for (k, row) in aug.iter_mut().enumerate() {
        if k != i {
            let factor = row[i];
            for (cell, &top) in row.iter_mut().zip(pivot_row.iter()) {
                *cell -= factor * top;
            }
        }
    }
    Ok(())
}

/// Per-column scale-aware pivot thresholds for Gauss-Jordan / Gaussian
/// elimination. The previous matrix-wide `‖A‖∞ · n · ε` formula clipped
/// matrices with disparate column magnitudes: `diag(1e200, 1e-200)` has a
/// legitimate inverse `diag(1e-200, 1e200)`, but its second pivot
/// (`1e-200`) sat far below the global threshold dominated by `1e200` and
/// got flagged as singular. Anchoring the threshold on each column's own
/// maximum restores the intrinsic rank-revealing property: a pivot is
/// "numerically zero" only if it's tiny *compared to its own column*.
///
/// Returns one threshold per column in `0..cols`. Callers only need the
/// entries corresponding to columns they'll actually pivot in.
fn gauss_jordan_column_thresholds(data: &[Vec<f64>], cols: usize) -> Vec<f64> {
    let n_f = num_traits::NumCast::from(data.len().max(1)).unwrap_or(1.0_f64);
    (0..cols)
        .map(|j| {
            let col_max = data.iter().map(|row| row[j].abs()).fold(0.0_f64, f64::max);
            // Guard all-zero columns so `pivot < threshold` stays triggerable
            // by a literal `0.0` (otherwise the threshold would be exactly
            // `0.0`, and `0.0 < 0.0` is false).
            (col_max * n_f * f64::EPSILON).max(f64::MIN_POSITIVE)
        })
        .collect()
}

#[must_use]
pub fn matrix_inverse(a: &str) -> String {
    let m = match Matrix::parse(TOOL_MATRIX_INVERSE, "a", a) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if m.rows != m.cols {
        return error_with_detail(
            TOOL_MATRIX_INVERSE,
            ErrorCode::InvalidInput,
            "inverse requires a square matrix",
            &format!("dim={}x{}", m.rows, m.cols),
        );
    }
    let n = m.rows;
    // Build augmented [A | I] — each row is the source row followed by an
    // identity row (1 in the right slot, zeros elsewhere).
    let mut aug: Vec<Vec<f64>> = m
        .data
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mut out = Vec::with_capacity(2 * n);
            out.extend_from_slice(row);
            out.extend((0..n).map(|j| if j == i { 1.0 } else { 0.0 }));
            out
        })
        .collect();
    // Per-column thresholds derived from the *original* `A` magnitudes — the
    // identity half is always O(1) and can't mislead the scale estimate.
    // Each column gets its own floor so `diag(1e200, 1e-200)` doesn't let
    // the first column's huge pivot bury the second column's legitimately
    // small one.
    let col_thresholds = gauss_jordan_column_thresholds(&m.data, n);
    for (i, &threshold) in col_thresholds.iter().enumerate().take(n) {
        if gauss_jordan_step(&mut aug, i, n, threshold).is_err() {
            return error(
                TOOL_MATRIX_INVERSE,
                ErrorCode::DomainError,
                "matrix is singular — inverse does not exist",
            );
        }
    }
    let data: Vec<Vec<f64>> = aug.into_iter().map(|row| row[n..].to_vec()).collect();
    let result = Matrix {
        rows: n,
        cols: n,
        data,
    };
    ok_matrix(TOOL_MATRIX_INVERSE, &result)
}

#[must_use]
pub fn matrix_trace(a: &str) -> String {
    let m = match Matrix::parse(TOOL_MATRIX_TRACE, "a", a) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if m.rows != m.cols {
        return error_with_detail(
            TOOL_MATRIX_TRACE,
            ErrorCode::InvalidInput,
            "trace requires a square matrix",
            &format!("dim={}x{}", m.rows, m.cols),
        );
    }
    let trace: f64 = m.data.iter().enumerate().map(|(i, row)| row[i]).sum();
    if !trace.is_finite() {
        return error_with_detail(
            TOOL_MATRIX_TRACE,
            ErrorCode::Overflow,
            "trace is non-finite (overflow/underflow to ±∞ or NaN)",
            &format!("trace={trace:?}"),
        );
    }
    Response::ok(TOOL_MATRIX_TRACE).result(fmt(trace)).build()
}

#[must_use]
pub fn matrix_rank(a: &str) -> String {
    let m = match Matrix::parse(TOOL_MATRIX_RANK, "a", a) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let r = compute_rank(m.data.clone(), m.rows, m.cols);
    Response::ok(TOOL_MATRIX_RANK).result(r.to_string()).build()
}

fn compute_rank(mut a: Vec<Vec<f64>>, rows: usize, cols: usize) -> usize {
    // Per-column scale-relative zero thresholds — matches `gauss_jordan_step`.
    // A `diag(1e200, 1e-200)` matrix has rank 2; a single matrix-wide floor
    // derived from `‖A‖∞` would wrongly zero out the tiny pivot.
    let col_thresholds = gauss_jordan_column_thresholds(&a, cols);
    let mut rank = 0;
    let mut row = 0;
    for col in 0..cols {
        if row >= rows {
            break;
        }
        let pivot = (row..rows)
            .max_by(|&x, &y| {
                a[x][col]
                    .abs()
                    .partial_cmp(&a[y][col].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(row);
        if a[pivot][col].abs() < col_thresholds[col] {
            continue;
        }
        a.swap(pivot, row);
        let pivot_value = a[row][col];
        let pivot_row = a[row].clone();
        for (k, current_row) in a.iter_mut().enumerate() {
            if k != row {
                let factor = current_row[col] / pivot_value;
                for (cell, &top) in current_row.iter_mut().zip(pivot_row.iter()).skip(col) {
                    *cell -= factor * top;
                }
            }
        }
        row += 1;
        rank += 1;
    }
    rank
}

/// Eigenvalues of a 2x2 matrix via the characteristic polynomial. For 3+x3+
/// matrices the user should reach for a numerical-linear-algebra library;
/// this tool is intentionally scoped.
#[must_use]
pub fn matrix_eigenvalues_2x2(a: &str) -> String {
    let m = match Matrix::parse(TOOL_MATRIX_EIGENVALUES_2X2, "a", a) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if m.rows != 2 || m.cols != 2 {
        return error_with_detail(
            TOOL_MATRIX_EIGENVALUES_2X2,
            ErrorCode::InvalidInput,
            "tool only supports 2x2 matrices",
            &format!("dim={}x{}", m.rows, m.cols),
        );
    }
    let a11 = m.data[0][0];
    let a12 = m.data[0][1];
    let a21 = m.data[1][0];
    let a22 = m.data[1][1];

    // Scale protection: `trace² - 4·det` overflows to `inf` once any entry
    // exceeds ~1e154, and underflows to `0` for entries below ~1e-154. A
    // symmetric matrix with tiny off-diagonals was therefore reported as
    // having `±∞` imaginary eigenvalues. Normalising by the max-abs entry
    // keeps every intermediate inside the f64 sweet spot; the eigenvalues
    // are scale-covariant, so we multiply back at the end.
    let scale = [a11, a12, a21, a22]
        .iter()
        .fold(0.0_f64, |acc, v| acc.max(v.abs()));
    if scale == 0.0 {
        return Response::ok(TOOL_MATRIX_EIGENVALUES_2X2)
            .field("KIND", "real")
            .field("LAMBDA1", fmt(0.0))
            .field("LAMBDA2", fmt(0.0))
            .build();
    }
    let inv_scale = 1.0 / scale;
    let b11 = a11 * inv_scale;
    let b12 = a12 * inv_scale;
    let b21 = a21 * inv_scale;
    let b22 = a22 * inv_scale;
    // λ² - (b11+b22)λ + (b11*b22 - b12*b21) = 0 for the scaled matrix.
    let trace_s = b11 + b22;
    let det_s = b11.mul_add(b22, -(b12 * b21));
    let disc_s = trace_s.mul_add(trace_s, -(4.0 * det_s));
    if disc_s < 0.0 {
        // Complex conjugate pair on the scaled matrix → eigenvalues of the
        // original matrix are `scale · (real ± imag·i)`.
        let real = (trace_s / 2.0) * scale;
        let imag = ((-disc_s).sqrt() / 2.0) * scale;
        return Response::ok(TOOL_MATRIX_EIGENVALUES_2X2)
            .field("KIND", "complex")
            .field("LAMBDA1", format!("{},{}", fmt(real), fmt(imag)))
            .field("LAMBDA2", format!("{},{}", fmt(real), fmt(-imag)))
            .build();
    }
    let s = disc_s.sqrt();
    let l1 = f64::midpoint(trace_s, s) * scale;
    let l2 = f64::midpoint(trace_s, -s) * scale;
    Response::ok(TOOL_MATRIX_EIGENVALUES_2X2)
        .field("KIND", "real")
        .field("LAMBDA1", fmt(l1))
        .field("LAMBDA2", fmt(l2))
        .build()
}

/// 3D cross product. Inputs are CSV "x,y,z".
#[must_use]
pub fn cross_product(a: &str, b: &str) -> String {
    let av = match parse_csv_f64(TOOL_CROSS_PRODUCT, "a", a) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let bv = match parse_csv_f64(TOOL_CROSS_PRODUCT, "b", b) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if av.len() != 3 || bv.len() != 3 {
        return error_with_detail(
            TOOL_CROSS_PRODUCT,
            ErrorCode::InvalidInput,
            "cross product requires 3D vectors",
            &format!("a.len={}, b.len={}", av.len(), bv.len()),
        );
    }
    let cx = av[1].mul_add(bv[2], -(av[2] * bv[1]));
    let cy = av[2].mul_add(bv[0], -(av[0] * bv[2]));
    let cz = av[0].mul_add(bv[1], -(av[1] * bv[0]));
    for (label, val) in [("x", cx), ("y", cy), ("z", cz)] {
        if !val.is_finite() {
            return error_with_detail(
                TOOL_CROSS_PRODUCT,
                ErrorCode::Overflow,
                "cross-product component is non-finite (overflow/underflow to ±∞ or NaN)",
                &format!("component={label}, value={val:?}"),
            );
        }
    }
    Response::ok(TOOL_CROSS_PRODUCT)
        .result(format!("{},{},{}", fmt(cx), fmt(cy), fmt(cz)))
        .build()
}

/// Solve `Ax = b` via Gaussian elimination with partial pivoting.
/// Classify a singular augmented system as inconsistent (`rank(A|b) > rank(A)`
/// — some row reduces to `0 = nonzero`) or underdetermined (ranks equal but
/// less than n — infinite solution family). The caller passes the untouched
/// augmented matrix so rank analysis is independent of the forward-elimination
/// state.
fn classify_singular_system(original: Vec<Vec<f64>>, n: usize) -> String {
    let a_only: Vec<Vec<f64>> = original.iter().map(|row| row[..n].to_vec()).collect();
    let rank_a = compute_rank(a_only, n, n);
    let rank_aug = compute_rank(original, n, n + 1);
    if rank_aug > rank_a {
        error(
            TOOL_GAUSSIAN_ELIMINATION,
            ErrorCode::DomainError,
            "system is inconsistent — no solution exists",
        )
    } else {
        error(
            TOOL_GAUSSIAN_ELIMINATION,
            ErrorCode::DomainError,
            "system is underdetermined — infinitely many solutions",
        )
    }
}

/// `coefficients` is the augmented matrix `[A | b]` in the same `;`/`,`
/// format used elsewhere — N rows × (N+1) cols.
#[must_use]
pub fn gaussian_elimination(coefficients: &str) -> String {
    let m = match Matrix::parse(TOOL_GAUSSIAN_ELIMINATION, "coefficients", coefficients) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if m.cols != m.rows + 1 {
        return error_with_detail(
            TOOL_GAUSSIAN_ELIMINATION,
            ErrorCode::InvalidInput,
            "augmented matrix must have N rows and N+1 columns",
            &format!("rows={}, cols={}", m.rows, m.cols),
        );
    }
    let n = m.rows;
    // Keep a copy of the raw augmented matrix so we can re-run rank analysis
    // to distinguish inconsistent vs underdetermined systems after a singular
    // pivot is detected below.
    let original = m.data.clone();
    let mut a = m.data;
    // Per-column scale-relative zero thresholds. A mixed-scale system like
    // `diag(1e200, 1e-200) · x = b` is well-conditioned and must solve
    // cleanly; a matrix-wide threshold would wrongly reject it as singular.
    let col_thresholds = gauss_jordan_column_thresholds(&a, n);
    for i in 0..n {
        let pivot = (i..n)
            .max_by(|&x, &y| {
                a[x][i]
                    .abs()
                    .partial_cmp(&a[y][i].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(i);
        if a[pivot][i].abs() < col_thresholds[i] {
            return classify_singular_system(original, n);
        }
        a.swap(pivot, i);
        let pivot_value = a[i][i];
        let pivot_row = a[i].clone();
        for row in a.iter_mut().skip(i + 1) {
            let factor = row[i] / pivot_value;
            for (cell, &top) in row.iter_mut().zip(pivot_row.iter()).skip(i) {
                *cell -= factor * top;
            }
        }
    }
    // Back-substitute
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let sum: f64 = a[i][i + 1..n]
            .iter()
            .zip(x[i + 1..].iter())
            .fold(a[i][n], |acc, (&aij, &xj)| aij.mul_add(-xj, acc));
        x[i] = sum / a[i][i];
    }
    let solution = x.iter().map(|v| fmt(*v)).collect::<Vec<_>>().join(",");
    Response::ok(TOOL_GAUSSIAN_ELIMINATION)
        .field("N", n.to_string())
        .field("SOLUTION", solution)
        .build()
}

fn parse_csv_f64(tool: &str, label: &str, input: &str) -> Result<Vec<f64>, String> {
    let mut out = Vec::new();
    for part in input.split(',') {
        let trimmed = part.trim();
        match trimmed.parse::<f64>() {
            Ok(v) if v.is_finite() => out.push(v),
            _ => {
                return Err(error_with_detail(
                    tool,
                    ErrorCode::ParseError,
                    "vector element is not a finite number",
                    &format!("{label}={trimmed}"),
                ));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_field(out: &str, key: &str, expected: f64) {
        let primary = format!(" | {key}: ");
        let header = format!(": OK | {key}: ");
        let part = out
            .split(&primary)
            .nth(1)
            .or_else(|| out.split(&header).nth(1))
            .unwrap_or_else(|| panic!("field {key} not found in `{out}`"));
        let value_str: String = part
            .chars()
            .take_while(|c| *c != ' ' && *c != '\n')
            .collect();
        let v: f64 = value_str.parse().expect("parse");
        assert!(
            (v - expected).abs() < 1e-6,
            "{key}: expected ~{expected}, got {v} in `{out}`"
        );
    }

    #[test]
    fn add_2x2() {
        let out = matrix_add("1,2;3,4", "5,6;7,8");
        assert!(out.contains("MATRIX: 6.0,8.0;10.0,12.0"), "got {out}");
    }

    #[test]
    fn add_dimension_mismatch_errors() {
        let out = matrix_add("1,2;3,4", "1,2,3;4,5,6");
        assert!(out.starts_with("MATRIX_ADD: ERROR"));
    }

    #[test]
    fn mult_2x2_identity() {
        // [1,2;3,4] * I = [1,2;3,4]
        let out = matrix_mult("1,2;3,4", "1,0;0,1");
        assert!(out.contains("MATRIX: 1.0,2.0;3.0,4.0"), "got {out}");
    }

    #[test]
    fn mult_compatible_dims() {
        // 2x3 * 3x2 = 2x2
        let out = matrix_mult("1,2,3;4,5,6", "1,2;3,4;5,6");
        // Row1: [1*1+2*3+3*5, 1*2+2*4+3*6] = [22, 28]
        // Row2: [4*1+5*3+6*5, 4*2+5*4+6*6] = [49, 64]
        assert!(out.contains("MATRIX: 22.0,28.0;49.0,64.0"), "got {out}");
    }

    #[test]
    fn transpose_2x3() {
        let out = matrix_transpose("1,2,3;4,5,6");
        assert!(out.contains("MATRIX: 1.0,4.0;2.0,5.0;3.0,6.0"), "got {out}");
    }

    #[test]
    fn determinant_2x2() {
        // det([1,2;3,4]) = 1*4 - 2*3 = -2
        approx_field(&matrix_determinant("1,2;3,4"), "RESULT", -2.0);
    }

    #[test]
    fn determinant_3x3() {
        // det([2,0,0;0,3,0;0,0,4]) = 24
        approx_field(&matrix_determinant("2,0,0;0,3,0;0,0,4"), "RESULT", 24.0);
    }

    #[test]
    fn determinant_singular() {
        // Two identical rows → det = 0
        approx_field(&matrix_determinant("1,2;1,2"), "RESULT", 0.0);
    }

    #[test]
    fn determinant_singular_fp_noise_snaps_to_zero() {
        // [[1,2,3],[4,5,6],[7,8,9]] is rank-deficient (R3 = 2*R2 - R1).
        // Gaussian elimination returns FP residue (~6.66e-16); the threshold
        // must snap it to exact 0 so the output is unambiguous.
        let out = matrix_determinant("1,2,3;4,5,6;7,8,9");
        assert!(out.contains("RESULT: 0.0") || out.contains("RESULT: 0"));
    }

    #[test]
    fn determinant_truly_small_not_clipped() {
        // A diagonal matrix with tiny but well-conditioned entries must not be
        // collapsed to zero — the scale-relative threshold protects it.
        let out = matrix_determinant("1e-5,0;0,1e-5");
        // expected 1e-10; must not be clipped.
        assert!(out.contains("e-10"), "got {out}");
    }

    #[test]
    fn determinant_tiny_entries_1e_minus_10_not_clipped() {
        // Regression: `diag(1e-10, 1e-10)` has the honest determinant
        // `1e-20`. The previous `scale.max(1.0) * 1e-12` threshold clamped
        // scale up to 1 and clipped this value to zero. The dimension-aware
        // `‖A‖ⁿ · n · ε` threshold drops to ~9e-36 for this matrix so the
        // result survives.
        let out = matrix_determinant("1e-10,0;0,1e-10");
        assert!(out.contains("1e-20"), "got {out}");
        assert!(!out.contains("RESULT: 0\n"), "got {out}");
    }

    #[test]
    fn determinant_non_square_errors() {
        let out = matrix_determinant("1,2,3;4,5,6");
        assert!(out.starts_with("MATRIX_DETERMINANT: ERROR"));
    }

    #[test]
    fn inverse_2x2() {
        // inv([1,2;3,4]) = [[-2,1],[1.5,-0.5]] — verify by extracting numbers
        let out = matrix_inverse("1,2;3,4");
        let matrix_part = out.split("MATRIX: ").nth(1).expect("matrix");
        let cells: Vec<f64> = matrix_part
            .split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();
        assert_eq!(cells.len(), 4);
        let expected = [-2.0, 1.0, 1.5, -0.5];
        for (i, exp) in expected.iter().enumerate() {
            assert!(
                (cells[i] - exp).abs() < 1e-9,
                "cell {i}: expected {exp}, got {} (full: {out})",
                cells[i]
            );
        }
    }

    #[test]
    fn inverse_singular_errors() {
        let out = matrix_inverse("1,2;1,2");
        assert!(out.starts_with("MATRIX_INVERSE: ERROR"));
    }

    #[test]
    fn inverse_tiny_entries_well_conditioned_not_rejected() {
        // `diag(1e-20, 1e-20)` has a legitimate inverse `diag(1e20, 1e20)`.
        // The previous fixed `1e-12` pivot threshold wrongly declared it
        // singular. The scale-relative threshold now keeps it accepted.
        let out = matrix_inverse("1e-20,0;0,1e-20");
        assert!(out.starts_with("MATRIX_INVERSE: OK"), "got {out}");
        assert!(out.contains("1e20"), "got {out}");
    }

    #[test]
    fn rank_tiny_entries_reports_full_rank() {
        // `diag(1e-20, 1e-20)` has rank 2 — the scale-relative zero
        // threshold must let both pivots through.
        let out = matrix_rank("1e-20,0;0,1e-20");
        assert!(out.contains("RESULT: 2"), "got {out}");
    }

    #[test]
    fn inverse_mixed_scale_diagonal_not_rejected() {
        // Regression: `diag(1e200, 1e-200)` has `det = 1` and inverse
        // `diag(1e-200, 1e200)`. The previous matrix-wide pivot threshold
        // was dominated by `1e200` and wrongly declared the matrix
        // singular. Per-column thresholds give each pivot its own scale.
        let out = matrix_inverse("1e200,0;0,1e-200");
        assert!(out.starts_with("MATRIX_INVERSE: OK"), "got {out}");
        assert!(out.contains("1e-200"), "got {out}");
        assert!(out.contains("1e200"), "got {out}");
    }

    #[test]
    fn determinant_mixed_scale_diagonal_not_snapped() {
        // `det(diag(1e200, 1e-200)) = 1`. Hadamard-bound threshold is
        // `2 · 1e200 · 1e-200 · ε = 2 · 1 · ε ≈ 4.4e-16`, so `1` survives.
        let out = matrix_determinant("1e200,0;0,1e-200");
        assert!(
            out.contains("RESULT: 1") || out.contains("RESULT: 1.0"),
            "got {out}"
        );
    }

    #[test]
    fn rank_mixed_scale_diagonal_reports_full_rank() {
        // `diag(1e200, 1e-200)` has rank 2 — per-column pivot thresholds
        // scale each column independently.
        let out = matrix_rank("1e200,0;0,1e-200");
        assert!(out.contains("RESULT: 2"), "got {out}");
    }

    #[test]
    fn gaussian_elimination_mixed_scale_not_rejected() {
        // Augmented `[1e200, 0 | 1e200; 0, 1e-200 | 1e-200]` solves to
        // `x = [1, 1]`. Per-column thresholds let the tiny pivot pass.
        let out = gaussian_elimination("1e200,0,1e200;0,1e-200,1e-200");
        assert!(out.starts_with("GAUSSIAN_ELIMINATION: OK"), "got {out}");
        assert!(out.contains("SOLUTION: 1.0,1.0"), "got {out}");
    }

    #[test]
    fn inverse_emits_clean_integers_instead_of_ulp_residue() {
        // Regression: Gauss-Jordan leaked `-1.9999999999999996` into the
        // output. The shared `fmt` helper now snaps near-integer cells, so
        // the textbook answer appears verbatim in the response.
        let out = matrix_inverse("1,2;3,4");
        assert!(out.contains("-2.0,1.0;1.5,-0.5"), "got: {out}");
        assert!(!out.contains("1.9999999999999996"), "got: {out}");
    }

    #[test]
    fn trace_3x3() {
        // tr(diag(1,2,3)) = 6
        approx_field(&matrix_trace("1,0,0;0,2,0;0,0,3"), "RESULT", 6.0);
    }

    #[test]
    fn rank_full_2x2() {
        let out = matrix_rank("1,2;3,4");
        assert!(out.contains("RESULT: 2"), "got {out}");
    }

    #[test]
    fn rank_singular_2x2() {
        let out = matrix_rank("1,2;2,4");
        assert!(out.contains("RESULT: 1"), "got {out}");
    }

    #[test]
    fn eigenvalues_diagonal_2x2() {
        // [[2,0],[0,3]] → eigenvalues 3, 2
        let out = matrix_eigenvalues_2x2("2,0;0,3");
        assert!(out.contains("KIND: real"), "got {out}");
    }

    #[test]
    fn eigenvalues_complex_2x2() {
        // [[0,-1],[1,0]] (rotation) → eigenvalues ±i
        let out = matrix_eigenvalues_2x2("0,-1;1,0");
        assert!(out.contains("KIND: complex"), "got {out}");
    }

    #[test]
    fn eigenvalues_large_entries_no_overflow() {
        // Regression: `[[1e200, 1], [1, 1e200]]` is real-symmetric with
        // near-equal eigenvalues `1e200 ± 1` (indistinguishable in f64).
        // The old formula squared the trace (`(2e200)² = +∞`) and reported
        // `LAMBDA: 1e200, ±∞` as complex. Scale-normalising before the
        // quadratic keeps every intermediate in range and recovers the
        // real eigenvalues.
        let out = matrix_eigenvalues_2x2("1e200,1;1,1e200");
        assert!(out.contains("KIND: real"), "got {out}");
        assert!(out.contains("1e200"), "got {out}");
        assert!(!out.contains("inf"), "got {out}");
    }

    #[test]
    fn eigenvalues_tiny_entries_no_underflow() {
        // Scale protection also works at the other end of the range.
        let out = matrix_eigenvalues_2x2("1e-200,0;0,1e-200");
        assert!(out.contains("KIND: real"), "got {out}");
        assert!(out.contains("1e-200"), "got {out}");
    }

    #[test]
    fn cross_product_basic() {
        // i × j = k → (1,0,0) × (0,1,0) = (0,0,1)
        let out = cross_product("1,0,0", "0,1,0");
        assert!(out.contains("RESULT: 0.0,0.0,1.0"), "got {out}");
    }

    #[test]
    fn cross_product_non_3d_errors() {
        let out = cross_product("1,2", "3,4");
        assert!(out.starts_with("CROSS_PRODUCT: ERROR"));
    }

    #[test]
    fn gaussian_elimination_2x3_solve() {
        // x + y = 3, 2x + 3y = 8 → x=1, y=2
        let out = gaussian_elimination("1,1,3;2,3,8");
        assert!(out.contains("SOLUTION: 1.0,2.0"), "got {out}");
    }

    #[test]
    fn gaussian_elimination_inconsistent_errors() {
        // x + y = 1, 2x + 2y = 3 → rank(A)=1, rank(A|b)=2 → no solution.
        let out = gaussian_elimination("1,1,1;2,2,3");
        assert!(out.starts_with("GAUSSIAN_ELIMINATION: ERROR"));
        assert!(out.contains("inconsistent"), "got {out}");
    }

    #[test]
    fn gaussian_elimination_underdetermined_errors() {
        // x + y = 1, 2x + 2y = 2 → rank(A)=rank(A|b)=1 < n → infinite solutions.
        let out = gaussian_elimination("1,1,1;2,2,2");
        assert!(out.starts_with("GAUSSIAN_ELIMINATION: ERROR"));
        assert!(out.contains("underdetermined"), "got {out}");
    }

    #[test]
    fn gaussian_elimination_canonicalizes_negative_zero() {
        // Regression: the 3×3 system `[[2,1,1|5],[1,-1,-1|-1],[1,2,1|6]]`
        // solves cleanly to `(4/3, 7/3, 0)`, but the back-substitution chain
        // for `z` goes through `(6 − 4/3 − 2·7/3) / 1` which f64 evaluates
        // to a `-0.0` result (legitimate IEEE-754 sign on a zero from
        // `a − a − b + b` cancellation). Without the `canonicalize_zero` in
        // `fmt`, the solution surfaced as `"-0.0"` — technically correct
        // bit-wise but surprising to anyone reading the output.
        let out = gaussian_elimination("2,1,1,5;1,-1,-1,-1;1,2,1,6");
        assert!(
            !out.contains("-0.0"),
            "gauss solution should not surface negative zero, got {out}"
        );
        assert!(
            out.contains("SOLUTION: 1.33333333333333,2.33333333333333,0.0"),
            "got {out}"
        );
    }
}
