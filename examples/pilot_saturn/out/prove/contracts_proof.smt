; B.A.S.E. Temporal Contract Proof
; Contract: vdp1_cmd_kick
; Steps: 2

(declare-const t0 Int)
(declare-const t1 Int)

(assert (= t0 0))
(assert (< t0 t1))
(assert (<= (- t1 t0) 3000))
(assert (<= (- t1 t0) 5000))

(check-sat)
(get-model)


; B.A.S.E. Temporal Contract Proof
; Contract: vdp2_tvmd
; Steps: 1

(declare-const t0 Int)

(assert (= t0 0))

(check-sat)
(get-model)


; B.A.S.E. Temporal Contract Proof
; Contract: smpc_cmd
; Steps: 2

(declare-const t0 Int)
(declare-const t1 Int)

(assert (= t0 0))
(assert (< t0 t1))
(assert (<= (- t1 t0) 3000))
(assert (<= (- t1 t0) 5000))

(check-sat)
(get-model)


