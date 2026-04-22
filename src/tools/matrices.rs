//! Matrix algebra — multiplication, inverse, determinant, decomposition.
//!
//! Matrices are passed as JSON-style nested CSV: `"1,2;3,4"` for a 2x2 matrix
//! (rows separated by `;`, columns by `,`). All values are `f64`. For exact
//! arithmetic on small matrices, callers can compose with `evaluateExact`.

use crate::mcp::message::{ErrorCode, Response, error, error_with_detail};

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
            .map(|row| {
                row.iter()
                    .map(|v| format!("{v:?}"))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect::<Vec<_>>()
            .join(";")
    }
}

fn fmt(value: f64) -> String {
    format!("{value:?}")
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
    Response::ok(TOOL_MATRIX_ADD)
        .field("DIM", format!("{}x{}", result.rows, result.cols))
        .field("MATRIX", result.format())
        .build()
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
    Response::ok(TOOL_MATRIX_MULT)
        .field("DIM", format!("{}x{}", result.rows, result.cols))
        .field("MATRIX", result.format())
        .build()
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
    Response::ok(TOOL_MATRIX_TRANSPOSE)
        .field("DIM", format!("{}x{}", result.rows, result.cols))
        .field("MATRIX", result.format())
        .build()
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
    let scale = matrix_frobenius_norm(&m.data);
    let det_raw = compute_det(m.data);
    // Snap near-zero determinants to exact zero. The Gaussian-elimination
    // path returns FP noise (e.g. 6.66e-16) for rank-deficient integer
    // matrices like [[1,2,3],[4,5,6],[7,8,9]] whose true determinant is 0.
    // Use a threshold proportional to the matrix's Frobenius norm so well-
    // scaled "genuinely small" determinants are not clipped.
    let det = if det_raw.abs() < scale.max(1.0) * 1e-12 {
        0.0
    } else {
        det_raw
    };
    Response::ok(TOOL_MATRIX_DETERMINANT)
        .result(fmt(det))
        .build()
}

fn matrix_frobenius_norm(data: &[Vec<f64>]) -> f64 {
    let sum_sq: f64 = data.iter().flat_map(|row| row.iter()).map(|v| v * v).sum();
    sum_sq.sqrt()
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
fn gauss_jordan_step(aug: &mut [Vec<f64>], i: usize, n: usize) -> Result<(), ()> {
    let pivot = (i..n)
        .max_by(|&x, &y| {
            aug[x][i]
                .abs()
                .partial_cmp(&aug[y][i].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(i);
    if aug[pivot][i].abs() < 1e-12 {
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
    for i in 0..n {
        if gauss_jordan_step(&mut aug, i, n).is_err() {
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
    Response::ok(TOOL_MATRIX_INVERSE)
        .field("DIM", format!("{n}x{n}"))
        .field("MATRIX", result.format())
        .build()
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
    const EPS: f64 = 1e-9;
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
        if a[pivot][col].abs() < EPS {
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
    // λ² - (a11+a22)λ + (a11*a22 - a12*a21) = 0
    let trace = a11 + a22;
    let det = a11.mul_add(a22, -(a12 * a21));
    let disc = trace.mul_add(trace, -(4.0 * det));
    if disc < 0.0 {
        // Complex conjugate pair
        let real = trace / 2.0;
        let imag = (-disc).sqrt() / 2.0;
        return Response::ok(TOOL_MATRIX_EIGENVALUES_2X2)
            .field("KIND", "complex")
            .field("LAMBDA1", format!("{},{}", fmt(real), fmt(imag)))
            .field("LAMBDA2", format!("{},{}", fmt(real), fmt(-imag)))
            .build();
    }
    let s = disc.sqrt();
    let l1 = f64::midpoint(trace, s);
    let l2 = f64::midpoint(trace, -s);
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
    Response::ok(TOOL_CROSS_PRODUCT)
        .result(format!("{},{},{}", fmt(cx), fmt(cy), fmt(cz)))
        .build()
}

/// Solve `Ax = b` via Gaussian elimination with partial pivoting.
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
    let mut a = m.data;
    for i in 0..n {
        let pivot = (i..n)
            .max_by(|&x, &y| {
                a[x][i]
                    .abs()
                    .partial_cmp(&a[y][i].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(i);
        if a[pivot][i].abs() < 1e-12 {
            return error(
                TOOL_GAUSSIAN_ELIMINATION,
                ErrorCode::DomainError,
                "system is singular or has no unique solution",
            );
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
    fn gaussian_elimination_singular_errors() {
        // x + y = 1, 2x + 2y = 3 → no solution
        let out = gaussian_elimination("1,1,1;2,2,3");
        assert!(out.starts_with("GAUSSIAN_ELIMINATION: ERROR"));
    }
}
