from __future__ import annotations

import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import capture
import capture_schema
from capture_compare import compare_checkpoint
from capture_schema import CaptureValidationError, PROTOCOL, load_manifest, validate_manifest
from test_capture import facts, manifest


class ContractTests(unittest.TestCase):
    def test_version_one_names_the_migration_keys(self):
        value = manifest()
        value['version'] = 1
        value.pop('extensions', None)
        value.pop('pacing', None)
        message = '; '.join(validate_manifest(value))
        for word in ('version 1', 'extensions', 'pacing'):
            self.assertIn(word, message)

    def test_pacing_and_extensions_are_required_and_strict(self):
        for key in ('pacing', 'extensions'):
            value = manifest()
            value.pop(key, None)
            self.assertTrue(validate_manifest(value))
        for pacing in ('fixed', 'realtime'):
            value = manifest()
            value['pacing'] = pacing
            self.assertEqual(validate_manifest(value), [])
        value['pacing'] = 'fast'
        self.assertTrue(validate_manifest(value))
        for key in ('clocks', 'settle'):
            value = manifest()
            value[key]['extensions'] = {}
            self.assertTrue(validate_manifest(value))

    def test_positive_guards_frame_one_and_negative_has_one_difference(self):
        root = Path(__file__).resolve().parents[2] / 'simple-node2d-movement/capture'
        positive = load_manifest(root / 'orbit.json')
        point = next(p for p in positive['checkpoints'] if p['frame'] == 1)
        actual = facts(1)
        actual.update(viewport=positive['viewport'])
        actual['nodes'] = [dict(n, position=[100, 0] if n['path'] == 'Icon' else [600, 1]) for n in point['nodes']]
        actual['regions'] = [dict(r, non_blank_fraction=1, dominant_colour=r['dominant_colour']['rgb']) for r in point['regions']]
        self.assertIn('nodes.Icon.position', [d['path'] for d in compare_checkpoint(positive, point, actual)])
        negative = load_manifest(root / 'orbit-empty-region.json')
        differences = []
        for point in negative['checkpoints']:
            actual = facts(point['frame'])
            actual.update(scenario=negative['scenario'], viewport=negative['viewport'])
            actual['nodes'] = [dict(n) for n in point['nodes']]
            actual['regions'] = [dict(r, non_blank_fraction=0 if r['name'] == 'empty-corner' else 1,
                                      dominant_colour=r['dominant_colour']['rgb']) for r in point['regions']]
            differences.extend(compare_checkpoint(negative, point, actual))
        self.assertEqual([d['path'] for d in differences], ['regions.empty-corner.non_blank_fraction'])

    def test_adapter_documentation_names_clock_audio_and_reentry_boundaries(self):
        readme = (Path(capture.__file__).parent / 'README.md').read_text()
        for phrase in ('`bevy_step_ns` pins `Time<Virtual>`', 'motion advances on `Time<Fixed>`',
                       "Godot's physics delta", 'both clocks are pinned', 'not the same clock',
                       'frame n\'s `_process`', 'returns false while the flag exists',
                       '`BevyAppSingleton` is mutably bound', 'reaches `BevyAppSingleton` from a callback', 'undefined'):
            self.assertIn(phrase, readme)


class ExtensionTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.adapter = self.root / 'audio'
        (self.adapter / 'schema').mkdir(parents=True)
        (self.adapter / 'schema/extension.json').write_text('{"type":"object"}')
        (self.adapter / 'schema/validate.py').write_text(
            "def validate(value):\n    return [] if value == {'cues': [None, {'gain': 0.5}]} else ['bad cues']\n")
        self.patcher = patch.object(capture_schema, 'ADAPTER_ROOT', self.root)
        self.patcher.start()
        self.addCleanup(self.patcher.stop)
        self.value = manifest()
        self.value['extensions'] = {'audio': {'cues': [None, {'gain': 0.5}]}}

    def test_extension_is_opaque_and_delegated_to_its_validator(self):
        original = copy.deepcopy(self.value)
        self.assertEqual(validate_manifest(self.value), [])
        self.assertEqual(self.value, original)
        self.value['extensions']['audio']['cues'] = []
        self.assertIn('bad cues', '; '.join(validate_manifest(self.value)))

    def test_missing_validator_or_schema_and_unsafe_names_are_rejected(self):
        for name in ('schema/validate.py', 'schema/extension.json'):
            path = self.adapter / name
            saved = path.read_text()
            path.unlink()
            self.assertTrue(validate_manifest(self.value))
            path.write_text(saved)
        for name in ('../audio', 'audio/other', 'rendering_2d', 'facts', 'diff', 'request', 'png'):
            self.value['extensions'] = {name: {}}
            self.assertTrue(validate_manifest(self.value))

    def test_extension_facts_preserve_raw_json_and_have_their_own_capability(self):
        output = self.root / 'evidence'
        output.mkdir()
        session = capture.CaptureSession(self.value, output)
        payload = '{ "nested" : [1, null, {"unknown":true}] }'
        session.accept('CAPTURE_EXT adapter=audio frame=0 ' + payload)
        self.assertEqual((output / 'frame-000000.audio.json').read_text(), payload + '\n')
        self.assertEqual(session.verdict()['capabilities']['audio'], 'untested')
        with self.assertRaises(CaptureValidationError):
            session.accept('CAPTURE_EXT adapter=audio frame=0 ' + payload)
        for line in ('CAPTURE_EXT adapter=missing frame=1 {}', 'CAPTURE_EXT adapter=audio frame=61 {}'):
            with self.assertRaises(CaptureValidationError):
                session.accept(line)

    def test_verdict_step_can_set_only_its_own_entry(self):
        session = capture.CaptureSession(self.value, self.root)
        (self.adapter / 'verdict.py').write_text("def verdict(value, output):\n    return {'audio': 'pass'}\n")
        session.evaluate_extensions()
        self.assertEqual(session.verdict()['capabilities']['audio'], 'pass')
        (self.adapter / 'verdict.py').write_text("def verdict(value, output):\n    return {'rendering_2d': 'pass'}\n")
        with self.assertRaises(CaptureValidationError):
            session.evaluate_extensions()

    def test_unfamiliar_prefixes_are_whole_tokens(self):
        session = capture.CaptureSession(self.value, self.root)
        for token in ('CAPTURE_READY_MORE', 'CAPTURE_FACTS_AUDIO', 'CAPTURE_ERROR_OTHER', 'CAPTURE_EXT_MORE', 'CAPTURE_ACK_MORE'):
            session.accept(token + ' arbitrary payload')
        self.assertFalse(session.ready)
        self.assertEqual(session.errors, [])
        self.assertEqual(session.seen, [])


class PacingTests(unittest.TestCase):
    def test_realtime_requests_exist_before_ready(self):
        with tempfile.TemporaryDirectory() as directory:
            value = manifest()
            value['pacing'] = 'realtime'
            session = capture.CaptureSession(value, Path(directory))
            requests = list(Path(directory).glob('*.request.json'))
            self.assertEqual(len(requests), len(value['checkpoints']))
            self.assertTrue(all(json.loads(p.read_text())['version'] == 2 for p in requests))
            session.accept('CAPTURE_READY scenario=orbit scene=res://level.tscn frame=0')
            actual = facts()
            actual['scene'] = 'res://level.tscn'
            session.accept('CAPTURE_FACTS frame=0 ' + json.dumps(actual))
            self.assertNotIn('scene', [d['path'] for d in session.diffs])

    def test_realtime_keeps_virtual_clock_and_reports_variable_physics_ticks(self):
        value = manifest()
        value['pacing'] = 'realtime'
        actual = facts(60)
        actual['physics_ticks'] = 37
        actual['fixed_elapsed_ns'] = 37 * PROTOCOL['fixed_step_ns']
        self.assertEqual(compare_checkpoint(value, value['checkpoints'][-1], actual), [])
        actual['virtual_elapsed_ns'] += 1
        self.assertIn('virtual_elapsed_ns', [d['path'] for d in compare_checkpoint(value, value['checkpoints'][-1], actual)])


class FeatureGuardTests(unittest.TestCase):
    def test_plain_cdylib_is_rejected_before_launch(self):
        from test_driver import ProcessTests
        case = ProcessTests()
        case.setUp()
        try:
            with patch.object(capture, 'REPOSITORY', case.root), \
                    patch.object(capture, 'require_capture_library', side_effect=CaptureValidationError('build with --features capture')), \
                    patch.object(capture.subprocess, 'Popen') as launch:
                code = capture.run_capture(manifest(), case.output, 'unused', 1)
            self.assertEqual(code, 2)
            launch.assert_not_called()
        finally:
            case.doCleanups()

    def test_inspector_checks_the_actual_debug_library_symbol(self):
        with tempfile.TemporaryDirectory() as directory:
            project = Path(directory)
            library = project / 'extension.dylib'
            library.touch()
            (project / 'rust.gdextension').write_text('[libraries]\nmacos.debug = "res://extension.dylib"\n')
            with patch.object(capture.platform, 'system', return_value='Darwin'), \
                    patch.object(capture.platform, 'machine', return_value='arm64'), \
                    patch.object(capture.ctypes, 'CDLL') as load:
                load.return_value.godot_bevy_capture_version.return_value = 2
                capture.require_capture_library(project)
                self.assertEqual(Path(load.call_args.args[0]).resolve(), library.resolve())
                load.return_value.godot_bevy_capture_version.return_value = 1
                with self.assertRaisesRegex(CaptureValidationError, 'capture'):
                    capture.require_capture_library(project)
                load.return_value = object()
                with self.assertRaisesRegex(CaptureValidationError, 'capture'):
                    capture.require_capture_library(project)

    def test_shared_launcher_checks_library_before_dispatch(self):
        source = (Path(capture.__file__).parent.parent / 'run_godot.rs').read_text()
        self.assertLess(source.index('--check-library'), source.index('command.exec()'))


class HandshakeTests(unittest.TestCase):
    def test_ack_must_follow_instruction_and_match_exactly_once(self):
        with tempfile.TemporaryDirectory() as directory:
            session = capture.CaptureSession(manifest(), Path(directory))
            with self.assertRaises(CaptureValidationError):
                session.accept('CAPTURE_ACK exit=0')
            session.instruction()
            with self.assertRaises(CaptureValidationError):
                session.accept('CAPTURE_ACK exit=0')
            session.accept('CAPTURE_ACK exit=1')
            with self.assertRaises(CaptureValidationError):
                session.accept('CAPTURE_ACK exit=1')

    def test_run_capture_rejects_v1_before_session_or_process_creation(self):
        value = manifest()
        value['version'] = 1
        del value['pacing']
        del value['extensions']
        with tempfile.TemporaryDirectory() as directory, patch.object(capture.subprocess, 'Popen') as launch:
            self.assertEqual(capture.run_capture(value, Path(directory), 'unused', 1), 2)
            launch.assert_not_called()
