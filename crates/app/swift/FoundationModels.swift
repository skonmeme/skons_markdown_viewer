// Bridge from the Rust app to Apple's Foundation Models framework, which can
// only be used from Swift (STANDARD §6.6, §11.1). The functions are exported
// with C names; `crates/app/src/afm.rs` declares them on the Rust side.

import Foundation
import FoundationModels

/// The answer of the model for one batch of blocks.
@Generable
struct ChosenBlocks {
    @Guide(description: "Numbers of the blocks related to the query; empty when none is related")
    var blocks: [Int]
}

/// Availability of the model on this Mac: 0 available, 1 Apple Intelligence
/// is turned off, 2 the Mac does not support it, 3 the model is not ready,
/// 4 unavailable for another reason.
@_cdecl("smv_afm_availability")
public func smvAfmAvailability() -> Int32 {
    switch SystemLanguageModel.default.availability {
    case .available:
        return 0
    case .unavailable(.appleIntelligenceNotEnabled):
        return 1
    case .unavailable(.deviceNotEligible):
        return 2
    case .unavailable(.modelNotReady):
        return 3
    case .unavailable:
        return 4
    }
}

/// The context window of the model on this Mac in tokens.
@_cdecl("smv_afm_context_size")
public func smvAfmContextSize() -> Int {
    SystemLanguageModel.default.contextSize
}

/// Called once per request with its identifier, a status and, for status 0,
/// the chosen numbers separated by commas. Statuses: 0 answered,
/// 1 cancelled, 2 the batch could not be handled (language, safety rules,
/// decoding), 3 the context window was exceeded, 4 the model is not ready,
/// 5 rate limited, 6 another error (the text describes it).
public typealias SMVAfmCallback = @convention(c) (UInt64, Int32, UnsafePointer<CChar>?) -> Void

final class SMVRequest {
    let task: Task<Void, Never>

    init(task: Task<Void, Never>) {
        self.task = task
    }
}

/// Asks the model which blocks of a prompt are related to its query. Returns
/// a request that must be released with `smv_afm_release`.
@_cdecl("smv_afm_choose")
public func smvAfmChoose(
    _ instructions: UnsafePointer<CChar>,
    _ prompt: UnsafePointer<CChar>,
    _ id: UInt64,
    _ callback: SMVAfmCallback
) -> UnsafeMutableRawPointer {
    let instructions = String(cString: instructions)
    let prompt = String(cString: prompt)
    let task = Task.detached {
        let (status, text) = await choose(instructions: instructions, prompt: prompt)
        if let text {
            text.withCString { callback(id, status, $0) }
        } else {
            callback(id, status, nil)
        }
    }
    return Unmanaged.passRetained(SMVRequest(task: task)).toOpaque()
}

private func choose(instructions: String, prompt: String) async -> (Int32, String?) {
    do {
        let session = LanguageModelSession(model: .default, instructions: instructions)
        let response = try await session.respond(to: prompt, generating: ChosenBlocks.self)
        try Task.checkCancellation()
        return (0, response.content.blocks.map(String.init).joined(separator: ","))
    } catch is CancellationError {
        return (1, nil)
    } catch let error as LanguageModelSession.GenerationError {
        switch error {
        case .exceededContextWindowSize:
            return (3, nil)
        case .assetsUnavailable:
            return (4, nil)
        case .rateLimited:
            return (5, nil)
        case .guardrailViolation, .refusal, .unsupportedLanguageOrLocale, .decodingFailure,
            .unsupportedGuide:
            return (2, nil)
        default:
            return (6, String(describing: error))
        }
    } catch {
        return (6, String(describing: error))
    }
}

@_cdecl("smv_afm_cancel")
public func smvAfmCancel(_ request: UnsafeMutableRawPointer) {
    Unmanaged<SMVRequest>.fromOpaque(request).takeUnretainedValue().task.cancel()
}

@_cdecl("smv_afm_release")
public func smvAfmRelease(_ request: UnsafeMutableRawPointer) {
    Unmanaged<SMVRequest>.fromOpaque(request).release()
}
