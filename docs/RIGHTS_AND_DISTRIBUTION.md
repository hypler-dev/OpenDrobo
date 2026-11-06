# Rights and distribution research

Research date: November 6, 2026. This is a U.S.-focused engineering release assessment, not a legal opinion or a finding of ownership. It separates the independently written application, third-party dependencies, recognizable hardware illustrations, vendor files and trademarks.

## What company closure changes

Company closure does not by itself put software, artwork or documentation in the public domain. Copyright has a statutory duration and can transfer separately from the operating business. The U.S. Copyright Office explains [copyright duration](https://www.copyright.gov/help/faq/faq-duration.html) and [ownership transfers](https://copyright.gov/help/faq/faq-assignment.html).

A concrete ownership lead exists. A [USPTO-hosted filing and bankruptcy sale order](https://ttabvue.uspto.gov/ttabvue/ttabvue-91253736-OPP-19.pdf) includes Drobo, Inc. among the debtors. The November 4, 2022 order addresses asset/IP sales and the Drobo name; the September 25, 2023 filing describes related trademark purchases and assignments involving Serene Investment Management and Storcentric DIP Lender. This is evidence that rights were treated as transferable assets, not evidence that they disappeared. It does **not** establish the present owner of every Drobo copyright, trademark, firmware image or design. Current ownership would require following the relevant sale schedules and subsequent assignments.

## Practical release path

| Material | Present approach | Remaining review |
| --- | --- | --- |
| OpenDrobo implementation | Publish independently written source under the repository's MIT license | Preserve provenance; do not include copied vendor code |
| Rust/npm dependencies | Retain exact selected-package licenses, copyrights and required notices | Review expressions, source-availability obligations and the actual shipped artifact; see [third-party review](THIRD_PARTY.md) |
| Original vendor driver, service and firmware | Analyze local lawful copies; public builds do not bundle these files | Redistribution requires its own rights basis; installed user copies do not license our redistribution |
| Extracted original icons and art | Keep ignored and outside default public packages | Adding “Open,” changing colors or changing an angle does not itself grant rights |
| Newly rendered hardware art | New image files depict recognizable hardware with OpenDrobo styling and labeling | New rendering is not a trademark, trade-dress, design-right or derivative-work clearance; assess the finished artwork |
| Drobo/BeyondRAID names and marks | Explain compatibility and independent project status without claiming affiliation | Project naming and public marketing still need a fact-specific trademark review |

OpenDrobo can continue compatibility research and independent implementation while avoiding distribution of original binaries and extracted images. A new picture or an “Open” prefix does not settle every rights question. If use of vendor code/art is essential, identify the present rights holder and seek a written grant; no outreach has been sent by this project.

## What dependency clearance means here

It usually means complying with already granted open-source licenses, not asking Drobo to license Rust or Tauri code. Retain the exact copyright and permission notices required by the selected MIT/BSD/ISC/Zlib and other licenses; Apache-2.0 also addresses license copies, applicable NOTICE material and modification notices. Review each expression rather than treating all packages as MIT. [Apache's license](https://www.apache.org/licenses/LICENSE-2.0) specifies its redistribution conditions.

The selected macOS graph includes five MPL-2.0 packages: `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext` and `selectors`. Their source-availability and notification duties need a release disposition even if the larger application remains MIT licensed. Mozilla explains that distributing an executable compiled from unchanged MPL code requires telling recipients where to obtain the covered source; independent new files need not all become MPL licensed. Retain the exact locked source archives and provide a practical source-access notice with the release. See [Mozilla's official FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/). The notice generator's graph is conservative provenance, not proof that every selected dependency contributes code to the final executable.

## Reverse engineering

[17 U.S.C. § 1201(f)](https://www.govinfo.gov/content/pkg/USCODE-2024-title17/html/USCODE-2024-title17-chap12-sec1201.htm) contains a conditional interoperability provision for independently created programs. It requires, among other things, a lawfully obtained right to use the program, a qualifying interoperability purpose and acts that do not infringe copyright or violate other applicable law. It is not blanket permission to republish original code, firmware or artwork. Contract, jurisdiction, patent and other issues are not resolved by citing this provision. Record original-file provenance and independent implementation evidence; obtain qualified review before relying on a legal exception for distribution.

## Trademark status

A business being inactive is not an automatic determination that its mark is free to use. The [USPTO's current guidance](https://tmep.uspto.gov/RDMS/TMEP/print?href=TMEP-1600d1e916.html&version=current) describes a presumption after three consecutive years of nonuse, with factual exceptions and intent relevant. No current Drobo-mark abandonment or comprehensive clearance search is claimed here. An affiliation disclaimer is useful product communication; it does not itself grant rights.

## Release decision

The dependency notice package and vendor-rights assessment are separate gates. An unresolved all-lock inventory entry is not proof that the package ships in the macOS app, and a complete notice bundle is not legal approval. Review the scoped evidence, final package contents, public assets and marketing before publication. No public release, ownership declaration or permission request is performed by this document.
