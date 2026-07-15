# B.A.S.E. — Estratégia Comercial

> *Behavioral ASIC Synthesis Engine*
> Licença: AGPLv3 — uso comercial permitido, modificações devem ser compartilhadas.
> Para uso proprietário sem compartilhamento: licença comercial disponível.

---

## Modelo de Licenciamento

| Uso | Licença | Custo |
|-----|---------|-------|
| Open source / pesquisa | AGPLv3 | Gratuito |
| Empresa ≤ 10 funcionários | AGPLv3 | Gratuito |
| Empresa > 10 funcionários (uso interno) | AGPLv3 | Gratuito (modificações devem ser públicas) |
| **Produto proprietário** (incorporar B.A.S.E. em produto fechado) | **Comercial** | **Consultar** |
| **Serviço gerenciado** (B.A.S.E. como backend SaaS) | **Comercial** | **Consultar** |

A AGPLv3 exige que modificações sejam distribuídas para usuários da rede. Para empresas que querem usar B.A.S.E. internamente sem publicar modificações, a licença AGPLv3 padrão já cobre — o requisito de compartilhamento só se aplica se o software for **disponibilizado como serviço de rede**.

---

## Mercado 1 — Preservação Industrial

### Problema
Máquinas industriais com 20-40 anos de vida útil têm ASICs que param de ser fabricados. Quando o ASIC queima, a máquina para. Fabricante original não existe mais.

### Solução B.A.S.E.
```bash
base pipeline firmware_do_ASIC.bin --disasm -o replacement_pcb/
# → PCB compatível com componentes modernos
# → Firmware sintético (HAL + drivers)
# → BOM com componentes disponíveis
```

### Clientes típicos
- Indústria automotiva (ECUs, sensores)
- Máquinas CNC / robótica
- Equipamentos médicos
- Automação industrial
- Aviação / defesa (sistemas legados)

### Precificação
| Serviço | Preço |
|---------|-------|
| Análise de firmware + relatório de viabilidade | R$ 5.000 |
| Port completo (PCB + firmware + validação) | R$ 30.000 — 150.000 |
| Contrato de suporte anual (atualizações de BOM) | R$ 10.000/ano |

---

## Mercado 2 — Forense / Segurança

### Problema
Analistas de segurança precisam entender o comportamento de firmware de IoT, roteadores, câmeras, e dispositivos embedded sem acesso ao código fonte.

### Solução B.A.S.E.
```bash
base analyze firmware.bin --disasm --dot -o analysis/
# → HardwareSpec com blocos funcionais
# → Event Graph causal (WRITE → DMA → IRQ)
# → Evidence DB com fatos observados
# → Contratos temporais verificados via SMT
```

### Clientes típicos
- Equipes de Red Team / CTI
- Laboratórios de IoT security
- Fabricantes de hardware (auditoria de fornecedores)
- Seguros cibernéticos (due diligence)

### Precificação
| Serviço | Preço |
|---------|-------|
| Análise forense de firmware (por dispositivo) | R$ 8.000 |
| Scan contínuo de firmware (assinatura mensal, 50 dispositivos/mês) | R$ 15.000/mês |
| Integração B.A.S.E. + SIEM / SOAR | R$ 30.000 |

---

## Mercado 3 — Educação / Pesquisa

### Problema
Cursos de engenharia reversa, arquitetura de computadores e sistemas embarcados precisam de ferramentas didáticas que mostrem a **essência** do hardware.

### Solução B.A.S.E.
```bash
# Alunos podem:
base analyze firmware.bin --disasm --dot -o lab/
# → Visualizar o grafo comportamental
# → Entender a relação entre firmware e hardware
# → Modificar o firmware e ver como a análise muda
```

### Clientes típicos
- Universidades (engenharia da computação, ciência da computação)
- Cursos técnicos de eletrônica
- Bootcamps de segurança
- Pesquisadores de paleocomputação

### Precificação
| Serviço | Preço |
|---------|-------|
| Licença educacional (por instituição, 50 alunos) | R$ 5.000/ano |
| Kit didático (B.A.S.E. + exemplos + apostila) | R$ 15.000 |
| Workshop presencial (2 dias) | R$ 20.000 |

---

## Mercado 4 — Serviço Gerenciado (SaaS)

### Problema
Pequenas e médias empresas não têm engenheiro de firmware especializado, mas precisam de análise de hardware.

### Solução B.A.S.E.
```text
Cliente envia firmware.zip
              ↓
Plataforma B.A.S.E. Cloud
              ↓
Relatório entregue em 24h:
  → hardware_spec.yaml
  → reference_design.yaml
  → BOM com componentes disponíveis
  → Relatório de tensão Ψ
```

### Precificação
| Plano | Preço | Inclui |
|-------|-------|--------|
| **Starter** — 1 análise/mês | R$ 500/mês | HardwareSpec + BOM |
| **Professional** — 10 análises/mês | R$ 3.000/mês | + PCB + firmware |
| **Enterprise** — ilimitado | R$ 15.000/mês | + Suporte prioritário + SLA 24h |

---

## Canais de Venda

| Canal | Como |
|-------|------|
| **GitHub Sponsors** | Doações recorrentes para desenvolvimento open source |
| **Parcerias com faculdades** | Licenciar para laboratórios de engenharia |
| **LinkedIn / Twitter** | Demonstrar casos de uso com hardwares reais (Power Mac, Amiga, etc.) |
| **Eventos de segurança** | BHack, Defcon, YSTS — show de reverse engineering |
| **Indicação** | Programa de afiliados para engenheiros que indicarem clientes |

---

## Próximo Passo Imediato

```bash
# 1. Adicionar badge de licença e contato comercial no README
# 2. Criar landing page (paleocomputacao.com.br)
# 3. Primeiro case study: Amiga CD32 ou Power Mac G5
# 4. Publicar no crates.io
```

Quer que eu implemente algum desses passos?
